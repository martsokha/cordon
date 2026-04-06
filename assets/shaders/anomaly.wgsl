#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

struct AnomalyParams {
    // 0=chemical, 1=thermal, 2=electric, 3=gravitational
    hazard_type: f32,
    intensity: f32,
    _padding1: f32,
    _padding2: f32,
}

@group(2) @binding(0) var<uniform> params: AnomalyParams;

fn hash(p: vec2<f32>) -> f32 {
    let k = vec2<f32>(0.3183099, 0.3678794);
    let q = p * k + k.yx;
    return fract(fract(q.x * q.y * (q.x + q.y)) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let world = in.world_position.xy;
    let t = globals.time;

    let center = uv - 0.5;
    let dist = length(center) * 2.0;
    let angle = atan2(center.y, center.x);

    // Soft circular fade
    let edge_fade = 1.0 - smoothstep(0.5, 1.0, dist);

    // Pulsing base
    let pulse = sin(t * 1.5) * 0.15 + 0.85;

    let hz = i32(params.hazard_type);
    var color: vec3<f32>;
    var alpha: f32;

    if hz == 0 {
        // Chemical: toxic green mist with bubbling pools
        let mist = fbm(world * 0.015 + vec2<f32>(t * 0.1, t * 0.07), 5);
        let pools = smoothstep(0.58, 0.62, fbm(world * 0.03 + vec2<f32>(t * 0.05, 0.0), 4));
        let drip = smoothstep(0.7, 0.75, noise(vec2<f32>(world.x * 0.04, world.y * 0.04 + t * 0.4)));
        color = mix(
            vec3<f32>(0.1, 0.3, 0.05),
            vec3<f32>(0.3, 0.6, 0.1),
            pools
        );
        color = mix(color, vec3<f32>(0.5, 0.8, 0.2), drip * 0.6);
        alpha = (mist * 0.3 + pools * 0.25 + drip * 0.15) * edge_fade * pulse;
    } else if hz == 1 {
        // Thermal: intense heat with rising flames and glowing ground
        let distort = noise(vec2<f32>(world.x * 0.02, world.y * 0.01 + t * 1.2));
        let haze = sin((world.y + distort * 50.0) * 0.08 + t * 4.0) * 0.5 + 0.5;
        let embers = smoothstep(0.78, 0.85, noise(world * 0.06 + vec2<f32>(t * 0.4, t * 1.5)));
        let flames = smoothstep(0.65, 0.75, fbm(world * 0.025 + vec2<f32>(0.0, t * 0.8), 4));
        let glow = smoothstep(0.4, 0.0, dist) * 0.3;
        let core_heat = smoothstep(0.6, 0.0, dist);
        color = mix(
            vec3<f32>(0.6, 0.15, 0.02),
            vec3<f32>(1.0, 0.5, 0.05),
            haze
        );
        color = mix(color, vec3<f32>(1.0, 0.9, 0.3), embers);
        color = mix(color, vec3<f32>(1.0, 0.6, 0.1), flames * 0.7);
        color += vec3<f32>(0.3, 0.05, 0.0) * core_heat;
        alpha = (haze * 0.25 + embers * 0.5 + flames * 0.3 + glow) * edge_fade * pulse;
    } else if hz == 2 {
        // Electric: arcing lightning with crackling static field
        let field = fbm(world * 0.02 + vec2<f32>(t * 0.6, t * -0.3), 4);
        // Lightning arcs along radial lines
        let arc_angle = angle * 3.0 + t * 2.0;
        let arc = smoothstep(0.05, 0.0, abs(sin(arc_angle) * dist - field * 0.5));
        // Random bright flashes
        let flash_seed = floor(t * 8.0);
        let flash = smoothstep(0.92, 0.95, hash(vec2<f32>(flash_seed, flash_seed * 1.7)));
        // Crackling static
        let crackle = smoothstep(0.72, 0.78, noise(world * 0.1 + vec2<f32>(t * 3.0, t * 2.0)));
        color = mix(
            vec3<f32>(0.1, 0.2, 0.5),
            vec3<f32>(0.4, 0.6, 1.0),
            field
        );
        color = mix(color, vec3<f32>(0.8, 0.9, 1.0), arc * 0.7 + crackle * 0.3);
        color += vec3<f32>(0.5, 0.5, 0.7) * flash;
        alpha = (field * 0.15 + arc * 0.35 + crackle * 0.2 + flash * 0.3) * edge_fade;
    } else {
        // Gravitational: dark vortex with swirling distortion rings
        let swirl_angle = angle + dist * 4.0 - t * 0.8;
        let rings = abs(sin(dist * 12.0 + swirl_angle * 2.0));
        let warp = fbm(world * 0.012 + vec2<f32>(cos(t * 0.3) * 0.5, sin(t * 0.3) * 0.5), 5);
        let void_center = smoothstep(0.3, 0.0, dist);
        // Particles being pulled inward
        let spiral = smoothstep(0.6, 0.65, noise(vec2<f32>(swirl_angle * 2.0, dist * 5.0 + t * 0.5)));
        color = mix(
            vec3<f32>(0.15, 0.05, 0.25),
            vec3<f32>(0.3, 0.1, 0.5),
            rings * 0.4
        );
        color = mix(color, vec3<f32>(0.05, 0.0, 0.1), void_center);
        color = mix(color, vec3<f32>(0.5, 0.3, 0.7), spiral * 0.4);
        alpha = (warp * 0.2 + rings * 0.1 + void_center * 0.25 + spiral * 0.15) * edge_fade * pulse;
    }

    alpha *= params.intensity * 0.3;

    if alpha < 0.003 {
        discard;
    }

    return vec4<f32>(color, alpha);
}
