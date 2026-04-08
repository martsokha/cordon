// Fog-of-war overlay. Sits over the map at z=4.5 (below clouds,
// above terrain) and renders three states:
//
//   - Inside any reveal circle               → fully transparent
//   - Scouted texel in scout_mask            → grey memory wash
//   - Neither                                → swirly dark cloud
//
// The reveal circles are passed in as a fixed-size uniform array
// (`counts.x` live entries). Long-term memory lives in a 256×256
// `R8Unorm` texture covering the 5000×5000 playable map — each
// texel is 0 (unscouted) or 1 (scouted), bilinear-filtered so
// the boundary between the two reads as a smooth gradient. The
// mask is monotonic: texels only go from 0 → 1, never back.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

const MAX_REVEALS: u32 = 32u;
// Matches `MAP_EXTENT` on the Rust side. Used to convert
// fragment world-space coordinates into mask UVs.
const MAP_EXTENT: f32 = 2500.0;

@group(2) @binding(0) var<uniform> counts: vec4<f32>;
@group(2) @binding(1) var<uniform> reveals: array<vec4<f32>, MAX_REVEALS>;
@group(2) @binding(2) var scout_mask: texture_2d<f32>;
@group(2) @binding(3) var scout_mask_sampler: sampler;

// Cheap hash → noise → fbm stack, same shape as terrain/clouds.
fn hash2(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let q = p * k + k.yx;
    let n = fract(q.x * q.y * (q.x + q.y));
    return fract(n * n * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Soft inside-circle test: returns 1.0 well inside, falling to 0.0
// at the rim with a short feather. Used for both reveal and
// discovered tests so the boundaries blend instead of clicking.
fn disk_visibility(
    world: vec2<f32>,
    centre: vec2<f32>,
    radius: f32,
    feather: f32,
) -> f32 {
    let d = distance(world, centre);
    return 1.0 - smoothstep(radius - feather, radius, d);
}

// Same but with a low-frequency noise warp on the *radius* so the
// boundary wobbles in/out instead of being a perfect circle. Used
// for the active reveal cones, where a hard ring is the giveaway
// that the player is looking at a fog mask.
fn disk_visibility_wobbly(
    world: vec2<f32>,
    centre: vec2<f32>,
    radius: f32,
    feather: f32,
    wobble_amp: f32,
) -> f32 {
    let offset = world - centre;
    let d = length(offset);
    // Sample noise in *world space* (not normalized) so the wobble
    // pattern stays put as the squad walks — the rim crinkles in
    // place rather than crawling around the squad like a ring of
    // ants.
    let warp = noise(centre * 0.02 + offset * 0.012) - 0.5;
    let wobbled_radius = radius + warp * wobble_amp;
    return 1.0 - smoothstep(wobbled_radius - feather, wobbled_radius, d);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world = in.world_position.xy;

    // ---- Currently-in-sight test. Walk the active reveal slots
    // and union their soft inside-tests. Reveal disks use the
    // wobbly variant so their rims aren't perfect circles — gives
    // each squad's vision cone an organic ragged edge.
    var visible = 0.0;
    let n_reveals = u32(counts.x);
    for (var i = 0u; i < MAX_REVEALS; i++) {
        if i >= n_reveals {
            break;
        }
        let r = reveals[i];
        visible = max(
            visible,
            disk_visibility_wobbly(world, r.xy, r.z, 35.0, r.z * 0.18),
        );
    }

    // ---- Memory test. Sample the persistent scout mask at the
    // fragment's world-space position. The mask covers
    // `[-MAP_EXTENT, MAP_EXTENT]` in both axes; remap to
    // `[0, 1]` UV space and sample.
    //
    // Naive bilinear sampling gives visible step edges along
    // the texture grid because four-of-a-kind texel neighbours
    // (all 0 or all 255) leave the bilinear falloff to fire
    // exactly at axis-aligned texel boundaries. To hide the
    // grid without upping the resolution we jitter the UV by
    // sub-texel world-space noise and take several samples,
    // then average. The eye blurs the jittered samples into a
    // smooth curve — same trick shadow maps and volumetric
    // fogs use.
    let mask_uv = (world / (MAP_EXTENT * 2.0)) + vec2<f32>(0.5);
    // 16-tap 4×4 grid blur across a ~3-texel radius. Sampling
    // on a fixed axis-aligned grid with no per-pixel rotation
    // avoids "hair" streaks from hash-based jitter, while a
    // wider kernel + more samples fully dissolves the grid
    // stepping into a smooth gradient. Fullscreen 16-tap is
    // still cheap for a once-per-frame overlay.
    let step = 1.0 / 1024.0;
    var sampled = 0.0;
    for (var iy = -2; iy < 2; iy++) {
        for (var ix = -2; ix < 2; ix++) {
            let offset = vec2<f32>(
                (f32(ix) + 0.5) * step,
                (f32(iy) + 0.5) * step,
            );
            sampled = sampled
                + textureSample(scout_mask, scout_mask_sampler, mask_uv + offset).r;
        }
    }
    let raw_memory = sampled * (1.0 / 16.0);
    // Wide smoothstep so the transition band spans multiple
    // output pixels — the scouted region fades in gradually
    // at the leading edge instead of reading as a sharp wave.
    let memory = smoothstep(0.05, 0.75, raw_memory);

    // Three qualitatively distinct states, differentiated by
    // both colour and alpha so the player can tell them apart
    // at a glance:
    //
    //   - Never seen:       dark desaturated grey at 0.85
    //                       alpha → terrain mostly occluded
    //                       by a dark wash, a hint of the
    //                       Zone shape bleeds through at 15%.
    //   - Memory (scouted): light desaturated grey at 0.30
    //                       alpha → terrain clearly visible,
    //                       subtle neutral tint signalling
    //                       "seen before, not currently live."
    //   - In sight:         fully transparent → the terrain
    //                       shows through unmodified.
    //
    // Bevy's `AlphaMode2d::Blend` is standard src-over, so the
    // output `(color, alpha)` lands on top of the terrain as
    //   terrain * (1 - alpha) + color * alpha
    // which means `color * alpha` is the additive contribution
    // and `1 - alpha` is how much terrain leaks through.
    let fog_color = vec3<f32>(0.02, 0.02, 0.03);
    let memory_color = vec3<f32>(0.02, 0.02, 0.03);
    let fog_alpha = 0.55;
    let memory_alpha = 0.32;

    // Lerp fog → memory based on how scouted this texel is.
    // Using `memory` as the mix factor means the colour and
    // alpha transition together, so the boundary between
    // "never seen" and "scouted" is a single continuous
    // gradient rather than two separate factors crossing over.
    var color = mix(fog_color, memory_color, memory);
    var alpha = mix(fog_alpha, memory_alpha, memory);

    // Live visibility cuts straight through both states. In
    // fully-lit pixels, `visible = 1` drives alpha to 0 and
    // the colour is irrelevant.
    alpha = alpha * (1.0 - visible);

    return vec4<f32>(color, clamp(alpha, 0.0, 1.0));
}
