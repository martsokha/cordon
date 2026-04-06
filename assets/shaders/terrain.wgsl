#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

// Simplex-ish hash noise
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

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 5; i++) {
        value += amplitude * noise(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.world_position.xy * 0.003;

    let n = fbm(uv);
    let detail = fbm(uv * 3.0 + 7.0);

    // Base terrain colors (very dark, desaturated Zone palette)
    let dark_green = vec3<f32>(0.05, 0.07, 0.04);
    let mid_green = vec3<f32>(0.07, 0.09, 0.06);
    let olive = vec3<f32>(0.08, 0.08, 0.06);
    let brown = vec3<f32>(0.07, 0.06, 0.05);

    // Blend terrain types
    var color = mix(dark_green, mid_green, n);
    color = mix(color, olive, smoothstep(0.4, 0.6, detail));
    color = mix(color, brown, smoothstep(0.55, 0.75, n * detail));

    // Subtle variation
    color += (detail - 0.5) * 0.04;

    return vec4<f32>(color, 1.0);
}
