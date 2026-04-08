// Fog-of-war overlay. Sits over the map at z=4.5 (below clouds,
// above terrain) and renders three states:
//
//   - Inside any reveal circle               → fully transparent
//   - Inside any discovered area disk        → grey memory wash
//   - Outside both                           → swirly dark cloud
//
// Reveal circles and discovered disks are passed in as fixed-size
// `array<vec4, N>` uniforms. The first `counts.x` / `counts.y`
// elements are valid; the rest are leftover slots and ignored.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

const MAX_REVEALS: u32 = 32u;
const MAX_DISCOVERED: u32 = 256u;

@group(2) @binding(0) var<uniform> counts: vec4<f32>;
@group(2) @binding(1) var<uniform> reveals: array<vec4<f32>, MAX_REVEALS>;
@group(2) @binding(2) var<uniform> discovered: array<vec4<f32>, MAX_DISCOVERED>;

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

fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < octaves; i++) {
        value += amplitude * noise(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
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
    let t = globals.time;
    let edge = max(abs(world.x), abs(world.y));

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

    // ---- Memory test. Discovered area disks behave the same way,
    // with a slightly wider feather since they're typically larger
    // and a hard rim would look weird against the surrounding fog.
    var memory = 0.0;
    let n_disc = u32(counts.y);
    for (var i = 0u; i < MAX_DISCOVERED; i++) {
        if i >= n_disc {
            break;
        }
        let d = discovered[i];
        memory = max(memory, disk_visibility(world, d.xy, d.z, 40.0));
    }

    // ---- Animated swirly cloud for unexplored regions. Two slow
    // fbm layers offset by time give a subtle drifting "fog of
    // war" texture without flickering. We only need this where
    // the fog is opaque, so the noise is cheap relative to the
    // terrain shader (3+2 octaves).
    let drift_a = vec2<f32>(t * 0.020, t * -0.015);
    let drift_b = vec2<f32>(t * -0.012, t * 0.018);
    let swirl_a = fbm(world * 0.0025 + drift_a, 3);
    let swirl_b = fbm(world * 0.006 + drift_b + 31.0, 2);
    let swirl = swirl_a * 0.65 + swirl_b * 0.35;

    // Pure-black fog palette — matches the laptop background
    // `Color::BLACK` rectangle underneath the terrain, so the
    // fog mesh and the background merge seamlessly past the
    // terrain edge. Only the brightest swirl crests lift above
    // pure black at all, giving a faint drifting texture.
    let dark = vec3<f32>(0.0, 0.0, 0.0);
    let mid = vec3<f32>(0.004, 0.004, 0.004);
    let highlight = vec3<f32>(0.012, 0.012, 0.012);
    var fog_color = mix(dark, mid, swirl);
    fog_color = mix(fog_color, highlight, smoothstep(0.55, 0.85, swirl) * 0.4);

    // Memory wash: a dim desaturated grey for "I've been here, but
    // can't see it now". Neutral grey, no colour tint, and kept
    // low so memorized corridors read as a subtle hint rather
    // than a bright trail.
    let memory_color = vec3<f32>(0.045, 0.045, 0.045);

    // Compose the three states. `visible` cuts straight through
    // the fog and the memory wash. `memory` only kicks in where
    // the area was *ever* seen but isn't currently in sight, so
    // we subtract `visible` from it.
    let mem_factor = clamp(memory - visible, 0.0, 1.0);
    let fog_factor = clamp(1.0 - max(visible, memory), 0.0, 1.0);

    // Final colour is a layered blend, alpha is the union of the
    // two opaque states. The visible cut-through gets pure 0.
    var color = fog_color * fog_factor + memory_color * mem_factor;
    let alpha = fog_factor * 0.92 + mem_factor * 0.55;

    // Edge-band darken: in the outer band before the terrain's
    // 2500 hard edge, crush the fog colour toward pure black so
    // the boundary fades into a solid black frame matching the
    // laptop background rectangle underneath. Alpha stays fully
    // opaque — we never let the terrain underneath show through.
    let edge_band = smoothstep(1900.0, 2500.0, edge);
    color *= 1.0 - edge_band;

    return vec4<f32>(color, alpha);
}
