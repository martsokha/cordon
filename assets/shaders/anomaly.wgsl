#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_sprite::mesh2d_view_bindings::globals

struct AnomalyParams {
    intensity: f32,
    _padding0: f32,
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

    // Soft circular fade so the disk has a clean falloff at the rim.
    let edge_fade = 1.0 - smoothstep(0.85, 1.0, dist);
    // Thin rim band so the boundary is a clean line, not a wide
    // gradient.
    let rim = smoothstep(0.93, 0.96, dist) * (1.0 - smoothstep(0.96, 0.99, dist));

    // A heavy void cloud — Zone-stuff bleeding out of the ground.
    // Two layers of fbm sampled with rotational shear so the cloud
    // slowly *swirls* without forming concentric rings. The center
    // darkens into a faint dark patch (not a hard black hole).
    // Reads as a drained, wrong-feeling patch of ground.
    //
    // Rotational shear: rotate the world coords by an angle that
    // varies with distance and time. This is what makes the cloud
    // look like it's being twisted, without producing geometric
    // rings.
    let shear_angle = t * 0.12 + dist * 1.4;
    let cs = cos(shear_angle);
    let sn = sin(shear_angle);
    let sheared = vec2<f32>(
        world.x * cs - world.y * sn,
        world.x * sn + world.y * cs,
    );

    let cloud_a = fbm(sheared * 0.012 + vec2<f32>(t * 0.05, t * -0.03), 3);
    let cloud_b = fbm(sheared * 0.025 - vec2<f32>(t * 0.04, 0.0), 3);
    let cloud = cloud_a * 0.65 + cloud_b * 0.35;

    // Soft dark center, no hard edge.
    let center_dim = 1.0 - smoothstep(0.0, 0.55, dist);

    // Desaturated bruise palette — corruption reads as sickly plum,
    // not paint.
    let base = vec3<f32>(0.07, 0.06, 0.10);     // deep slate
    let mid_col = vec3<f32>(0.16, 0.13, 0.20);  // faded plum
    let highlight = vec3<f32>(0.26, 0.22, 0.32);// dim lavender
    var color = mix(base, mid_col, cloud);
    color = mix(color, highlight, smoothstep(0.55, 0.85, cloud) * 0.5);
    // Darken toward the center — not a black hole, just a sink.
    color *= 1.0 - center_dim * 0.55;

    // Muted plum rim.
    color = mix(color, vec3<f32>(0.22, 0.16, 0.28), rim * 0.6);

    var alpha = (0.22 + cloud * 0.18 + center_dim * 0.15) * edge_fade
              + rim * 0.26;

    // Intensity controls overall vividness. Tuned subtle — even
    // high-tier anomalies sit on the ground, they don't shout.
    alpha *= 0.28 + params.intensity * 0.45;

    if alpha < 0.005 {
        discard;
    }

    return vec4<f32>(color, clamp(alpha, 0.0, 1.0));
}
