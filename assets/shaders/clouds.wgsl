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

struct DayNight {
    day_progress: f32,
}
@group(2) @binding(0) var<uniform> day_night: DayNight;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let world = in.world_position.xy;

    // Clamp to terrain bounds (5000x5000, ±2500)
    let edge = max(abs(world.x), abs(world.y));
    if edge > 2500.0 {
        discard;
    }
    let terrain_fade = 1.0 - smoothstep(2000.0, 2500.0, edge);

    // Large slow-moving cloud mass
    let uv1 = world * 0.003 + vec2<f32>(t * 0.04, t * 0.01);
    let n1 = fbm(uv1, 6);

    // Medium wispy layer drifting at different angle
    let uv2 = world * 0.007 + vec2<f32>(t * 0.03, t * 0.015) + 17.0;
    let n2 = fbm(uv2, 5);

    // Small fast detail wisps
    let uv3 = world * 0.015 + vec2<f32>(t * 0.06, t * 0.02) + 41.0;
    let n3 = fbm(uv3, 4);

    // Combine: base shape from large, edges from medium, texture from small
    let shape = n1 * 0.5 + n2 * 0.3 + n3 * 0.2;

    // Cloud density with soft edges (wider smoothstep = softer clouds)
    let density = smoothstep(0.42, 0.62, shape);

    // Thinner wispy edges
    let wisp = smoothstep(0.38, 0.55, shape) - density;

    // Day/night cycle synced with game time
    let noon_dist = abs(day_night.day_progress - 0.5) * 2.0;
    let day_cycle = 1.0 - noon_dist;

    // Cloud color shifts with time of day
    let night_shadow = vec3<f32>(0.15, 0.15, 0.25);
    let night_highlight = vec3<f32>(0.25, 0.25, 0.35);
    let day_shadow = vec3<f32>(0.55, 0.55, 0.60);
    let day_highlight = vec3<f32>(0.85, 0.85, 0.90);

    let shadow = mix(night_shadow, day_shadow, day_cycle);
    let highlight = mix(night_highlight, day_highlight, day_cycle);
    let cloud_color = mix(shadow, highlight, smoothstep(0.45, 0.6, shape));

    // Clouds more visible at night (backlit by moon)
    let alpha_mod = mix(0.5, 0.35, day_cycle);
    let alpha = (density * alpha_mod + wisp * 0.12) * terrain_fade;

    if alpha < 0.005 {
        discard;
    }

    return vec4<f32>(cloud_color, alpha);
}
