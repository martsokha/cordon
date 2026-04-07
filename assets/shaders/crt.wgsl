#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

fn hash(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let q = p * k + k.yx;
    return fract(fract(q.x * q.y * (q.x + q.y)) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = globals.time;

    // Scanlines
    let scan_freq = 400.0;
    let scanline = sin(uv.y * scan_freq * 3.14159) * 0.5 + 0.5;
    let scan_alpha = scanline * 0.06;

    // Subtle barrel distortion vignette
    let center = uv - 0.5;
    let dist = dot(center, center);
    let vignette = smoothstep(0.5, 0.2, dist);
    let vignette_alpha = (1.0 - vignette) * 0.15;

    // Radio static: random noise that flickers
    let static_uv = floor(uv * vec2<f32>(200.0, 150.0));
    let static_noise = hash(static_uv + vec2<f32>(t * 73.7, t * 31.3));

    // Occasional interference bursts (every ~8 seconds, lasts ~0.3s)
    let burst_cycle = fract(t * 0.125);
    let burst = smoothstep(0.0, 0.02, burst_cycle) * (1.0 - smoothstep(0.02, 0.06, burst_cycle));

    // Horizontal tear line that moves down
    let tear_y = fract(t * 0.3);
    let tear = 1.0 - smoothstep(0.0, 0.008, abs(uv.y - tear_y));
    let tear_alpha = tear * 0.08;

    // Combine
    let static_alpha = static_noise * burst * 0.25 + static_noise * 0.008;
    let total_alpha = scan_alpha + vignette_alpha + static_alpha + tear_alpha;

    // Green-tinted static
    let static_color = vec3<f32>(0.1, 0.2, 0.1);
    let scan_color = vec3<f32>(0.0, 0.0, 0.0);

    let color = mix(scan_color, static_color, static_alpha / max(total_alpha, 0.001));

    if total_alpha < 0.002 {
        discard;
    }

    return vec4<f32>(color, total_alpha);
}
