// CCTV monitor screen — samples the render-target image fed by the
// antechamber camera and slaps the "old security monitor" look on
// top: scanlines, faint grain, vignette, and a slight tint pull
// toward sickly green-grey.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct CctvParams {
    // vec4-sized to satisfy WGSL 16-byte uniform alignment.
    // .x = effect strength multiplier. 1.0 for the small corner
    //      monitor (effects read strong at distance), ~0.25 for the
    //      fullscreen plane (effects would be overwhelming at full
    //      intensity when covering the whole screen).
    values: vec4<f32>,
}

// Custom 3D material bindings live at group 3 in Bevy 0.18 —
// group 2 is reserved for the mesh storage buffer, and colliding
// with it produces a `Storage class Storage doesn't match Uniform`
// wgpu validation error.
@group(3) @binding(0) var<uniform> params: CctvParams;
@group(3) @binding(1) var feed_tex: texture_2d<f32>;
@group(3) @binding(2) var feed_sampler: sampler;

// IQ-style integer hash. Much better spectral behavior than the
// cheap fract-of-fract hash we were using before — the old one had
// visible low-frequency diagonal correlation that showed up as a
// drifting crescent once the grain was scaled up for the CRT look.
fn hash(p: vec2<f32>) -> f32 {
    let i = vec2<u32>(bitcast<u32>(p.x), bitcast<u32>(p.y));
    var n = i.x * 1597334677u + i.y * 3812015801u;
    n = (n ^ (n >> 16u)) * 2246822519u;
    n = (n ^ (n >> 13u)) * 3266489917u;
    n = n ^ (n >> 16u);
    return f32(n) / 4294967295.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = in.uv;
    let t = globals.time;
    // Effect strength: 1.0 for corner monitor, <1 for fullscreen.
    let s = params.values.x;

    // ---- Camera barrel-distortion: pull pixels toward the center
    // so the feed bows outward at the edges, like a CCTV lens.
    let centered = uv - 0.5;
    let r2 = dot(centered, centered);
    let bow = 1.0 + r2 * 0.35 * s;
    uv = 0.5 + centered * bow;
    uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));

    // ---- Sample the camera feed.
    var color = textureSample(feed_tex, feed_sampler, uv).rgb;

    // ---- Pull the palette toward dim green-grey, like an old
    // phosphor monitor.
    let luma = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    let phosphor = vec3<f32>(luma * 0.85, luma * 1.05, luma * 0.80);
    color = mix(color, phosphor, 0.6 * s);

    // ---- Scanlines: dark horizontal stripes, held well below the
    // source resolution (288 rows) to avoid moiré interference.
    let scan = 1.0 - (0.22 * s) * (0.5 - 0.5 * sin(uv.y * 60.0 * 3.14159));
    color *= scan;

    // ---- Per-pixel grain. Quantize UV to the render-target grid
    // and use an integer frame counter (no directional bias) to
    // animate — this guarantees the noise has no low-frequency
    // structure, so it stays as static and never drifts as a blob.
    // Fades toward the edges so the barrel-distorted corners (where
    // texture sampling is stretched and the grain gets amplified)
    // stay calm.
    let pixel = floor(in.uv * vec2<f32>(512.0, 288.0));
    let frame = floor(t * 30.0);
    let edge_fade = 1.0 - smoothstep(0.25, 0.5, length(centered));
    let grain = (hash(pixel + vec2<f32>(frame, frame * 0.73)) - 0.5)
        * 0.05 * s * edge_fade;
    color += vec3<f32>(grain);

    // ---- Vignette: darker corners only.
    let vignette = 1.0 - smoothstep(0.4, 0.95, length(centered) * 1.4);
    color *= 1.0 - (1.0 - vignette) * 0.4 * s;

    // ---- Jitter pulse: brief dark dip every ~9 seconds, like a
    // weak signal flinching.
    let jitter_pulse = max(0.0, sin(t * 0.7) - 0.85) * 12.0;
    color *= 1.0 - jitter_pulse * 0.35 * s;

    // ---- Radio static burst: periodic green noise flashes.
    let static_pixel = floor(uv * vec2<f32>(200.0, 150.0));
    let static_noise = hash(static_pixel + vec2<f32>(frame * 1.7, frame * 2.3));
    let burst_cycle = fract(t * 0.125);
    let burst = smoothstep(0.0, 0.02, burst_cycle)
        * (1.0 - smoothstep(0.02, 0.06, burst_cycle));
    let static_alpha = (static_noise * burst * 0.4 + static_noise * 0.03) * s;
    color = mix(color, vec3<f32>(0.1, 0.2, 0.1), static_alpha);

    return vec4<f32>(color, 1.0);
}
