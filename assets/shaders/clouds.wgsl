#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

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
    for (var i = 0; i < 6; i++) {
        value += amplitude * noise(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let world = in.world_position.xy;

    // Large base layer
    let uv1 = world * 0.006 + vec2<f32>(t * 0.08, t * 0.02);
    // Medium layer drifting at a different angle
    let uv2 = world * 0.012 + vec2<f32>(t * 0.06, t * 0.03) + 13.0;
    // Small detail layer, fastest
    let uv3 = world * 0.025 + vec2<f32>(t * 0.12, t * 0.04) + 37.0;

    let n1 = fbm(uv1);
    let n2 = fbm(uv2);
    let n3 = fbm(uv3);
    let combined = n1 * 0.4 + n2 * 0.35 + n3 * 0.25;
    let cloud = smoothstep(0.48, 0.58, combined);

    // Fade on edges
    let edge = max(abs(world.x), abs(world.y));
    let edge_fade = 1.0 - smoothstep(4000.0, 6000.0, edge);

    let white = vec3<f32>(0.88, 0.88, 0.92);
    let alpha = cloud * edge_fade * 0.4;

    if alpha < 0.01 {
        discard;
    }

    return vec4<f32>(white, alpha);
}
