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

fn hash2(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        hash(p),
        hash(p + vec2<f32>(17.13, 31.71)),
    );
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

// Voronoi: returns (distance to nearest cell center, distance to second
// nearest). The gap between the two is what gives the bubble-cell border.
fn voronoi(p: vec2<f32>, t: f32) -> vec2<f32> {
    let i = floor(p);
    let f = fract(p);
    var d1 = 8.0;
    var d2 = 8.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let g = vec2<f32>(f32(dx), f32(dy));
            let cell = i + g;
            // Animate cell centers slowly so the pools breathe.
            let h = hash2(cell);
            let center = g + 0.5 + 0.4 * sin(t * 0.6 + h * 6.28);
            let r = center - f;
            let d = dot(r, r);
            if d < d1 {
                d2 = d1;
                d1 = d;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    return vec2<f32>(sqrt(d1), sqrt(d2));
}

// Distance from point `p` to the line segment `a`-`b`.
fn seg_dist(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let world = in.world_position.xy;
    let t = globals.time;

    let center = uv - 0.5;
    let dist = length(center) * 2.0;
    let angle = atan2(center.y, center.x);

    // Soft circular fade so the disk has a clean falloff at the rim.
    let edge_fade = 1.0 - smoothstep(0.85, 1.0, dist);
    // Thin rim band — three times tighter than before so the boundary
    // is a clean line, not a wide gradient.
    let rim = smoothstep(0.93, 0.96, dist) * (1.0 - smoothstep(0.96, 0.99, dist));

    let hz = i32(params.hazard_type);
    var color: vec3<f32>;
    var alpha: f32;

    if hz == 0 {
        // ---- Chemical ----
        // Drifting oil-slick film: a slow low-freq fbm wash with
        // fine granular high-freq noise on top. No cells, no bubbles
        // — just a sickly sheen flowing across the ground. Murky
        // olive over warm brown so it reads as toxic standing water,
        // not paint.
        let drift = vec2<f32>(t * 0.04, t * 0.03);
        let slick = fbm(world * 0.018 + drift, 4);
        let grain = fbm(world * 0.18 - drift * 2.0, 2);

        let base = vec3<f32>(0.14, 0.13, 0.06);     // muddy brown
        let mid_col = vec3<f32>(0.22, 0.22, 0.08);  // sickly olive
        let film = vec3<f32>(0.30, 0.28, 0.12);     // slick highlight
        color = mix(base, mid_col, slick);
        color = mix(color, film, smoothstep(0.55, 0.75, slick));
        // Granular surface darkens unevenly so the film looks textured.
        color *= 0.85 + grain * 0.25;

        // Rim: faded olive, very subtle.
        color = mix(color, vec3<f32>(0.32, 0.32, 0.14), rim * 0.55);

        alpha = (0.22 + slick * 0.18) * edge_fade
              + rim * 0.22;
    } else if hz == 1 {
        // ---- Thermal ----
        // Radial fire: solid radial gradient with sparks streaming
        // along the radial axis from rim toward center. No upward
        // motion — direction is purely along the radius so the whole
        // disk reads as a sink pulling embers in.
        let core = 1.0 - smoothstep(0.0, 0.55, dist);
        let mid = 1.0 - smoothstep(0.2, 0.85, dist);
        let pulse = sin(t * 3.0) * 0.22 + 0.78;
        let flicker = noise(vec2<f32>(t * 6.0, t * 4.5)) * 0.3 + 0.7;

        // Sparks: x = angular position (so sparks follow radial
        // streaks), y = radial position pulled inward over time. The
        // angular jitter is noise on the angle so streaks aren't
        // perfectly straight.
        let spark_angle = angle * 2.5 + noise(vec2<f32>(angle * 4.0, t * 0.8)) * 0.9;
        let spark_uv = vec2<f32>(
            spark_angle,
            dist * 14.0 - t * 5.0,
        );
        let spark = smoothstep(0.86, 0.96, noise(spark_uv)) * mid;

        // Wider, slower secondary sparks for the brighter core area.
        let core_spark_uv = vec2<f32>(
            spark_angle * 0.6,
            dist * 22.0 - t * 7.5,
        );
        let core_spark = smoothstep(0.90, 0.98, noise(core_spark_uv)) * core;

        // Palette: deep brick → amber → warm cream core.
        let deep = vec3<f32>(0.20, 0.05, 0.02);
        let hot = vec3<f32>(0.62, 0.26, 0.06);
        let white_hot = vec3<f32>(0.78, 0.60, 0.32);
        color = mix(deep, hot, mid);
        color = mix(color, white_hot, core * pulse * flicker);
        color += vec3<f32>(0.85, 0.50, 0.15) * spark * 0.75;
        color += vec3<f32>(0.95, 0.70, 0.25) * core_spark * 0.9;

        // Rim: deep brick.
        color = mix(color, vec3<f32>(0.45, 0.12, 0.03), rim * 0.7);

        alpha = (0.26 + mid * 0.20 + core * 0.30 * pulse + spark * 0.45 + core_spark * 0.5) * edge_fade
              + rim * 0.30;
    } else if hz == 2 {
        // ---- Electric ----
        // Lightning bolts originate at the center and walk *outward*
        // to a random landing point on the rim, so every visible
        // strike unambiguously touches the area boundary. Strike
        // bucket re-rolls bolt count and rim landing angles ~2.5
        // times per second so the pattern never repeats.
        let field = fbm(world * 0.04 + vec2<f32>(t * 0.4, t * -0.2), 3);
        let bg = mix(
            vec3<f32>(0.03, 0.04, 0.08),
            vec3<f32>(0.06, 0.07, 0.12),
            field
        );
        color = bg;

        // Strike bucket changes ~4 times per second. The bucket id is
        // mixed *multiplicatively* into per-bolt seeds (not added) so
        // bolts don't trace a straight line through hash space —
        // additive seeding caused bolt 0 to always land at the same
        // angle because hash(p) has axis-aligned aliasing.
        let strike_bucket = floor(t * 1.2);
        let bucket_hash_a = hash(vec2<f32>(strike_bucket, 13.7));
        let bucket_hash_b = hash(vec2<f32>(strike_bucket * 2.31, 91.3));
        let bolt_count_f = 2.0 + floor(bucket_hash_a * 4.0);
        let bolt_count = i32(bolt_count_f);

        // Rim landing radius slightly inside the visual rim so
        // perturbation can't push segment endpoints past it.
        let landing_radius = 0.46;

        var arc_acc = 0.0;
        for (var i = 0; i < 6; i++) {
            if i >= bolt_count {
                break;
            }
            let fi = f32(i);
            // Each bolt's seed combines the two bucket hashes with
            // the bolt index in a way that's not a straight line in
            // hash-space, plus an even angular slot offset so bolts
            // in the same bucket spread around the rim instead of
            // clustering. The slot offset gets a per-bucket phase
            // jitter so the spread isn't a fixed pattern either.
            let slot = (fi + bucket_hash_b * f32(bolt_count)) / f32(bolt_count);
            let bolt_seed = vec2<f32>(
                fract(bucket_hash_a * 17.13 + fi * 0.731),
                fract(bucket_hash_b * 31.71 + fi * 0.917),
            );

            // Random landing angle = even slot + per-bolt jitter so
            // bolts in the same bucket are spread around the disk
            // but still randomized within their slot.
            let jitter = (hash(bolt_seed) - 0.5) * 0.6;
            let rim_angle = (slot + jitter / 6.2832) * 6.2832;
            let landing = vec2<f32>(cos(rim_angle), sin(rim_angle)) * landing_radius;

            // Per-bolt flash. Floor is 0 so dim bolts fully vanish.
            let phase = t * (2.5 + hash(bolt_seed + vec2<f32>(0.0, 1.0)) * 1.5)
                      + hash(bolt_seed) * 6.2832;
            let flash_raw = sin(phase) * 0.5 + 0.5;
            let flash = pow(flash_raw, 6.0);

            // 5 jagged segments from center → rim landing point.
            // Perturbation peaks mid-bolt and tapers to 0 at both
            // ends so the bolt starts at center and ends *exactly*
            // at the landing point.
            var prev = vec2<f32>(0.0, 0.0);
            for (var j = 1; j <= 5; j++) {
                let fj = f32(j) / 5.0;
                let waypoint = mix(vec2<f32>(0.0, 0.0), landing, fj);
                // Symmetric taper: 0 at fj=0 and fj=1, peak at 0.5.
                // Lower amplitude than before so segments can't
                // overshoot the landing radius.
                let perturb_amt = 0.10 * sin(fj * 3.1415);
                let n = vec2<f32>(
                    noise(bolt_seed + vec2<f32>(fj * 7.0, t * 5.0)) - 0.5,
                    noise(bolt_seed + vec2<f32>(t * 5.0, fj * 7.0)) - 0.5,
                ) * perturb_amt;
                // Last segment endpoint is the exact landing point.
                var next = waypoint + n;
                if j == 5 {
                    next = landing;
                }
                let d = seg_dist(center, prev, next);
                arc_acc += smoothstep(0.012, 0.0, d) * flash;
                prev = next;
            }
        }
        let arcs = clamp(arc_acc, 0.0, 1.5);

        // Occasional full-disc flash for the whole anomaly.
        let global_flash_seed = floor(t * 3.0);
        let global_flash = smoothstep(0.96, 0.99, hash(vec2<f32>(global_flash_seed, global_flash_seed * 1.7)));

        // Muted steel-blue arcs.
        color = mix(color, vec3<f32>(0.40, 0.50, 0.65), clamp(arcs, 0.0, 1.0));
        color += vec3<f32>(0.45, 0.55, 0.68) * arcs * 0.35;
        color += vec3<f32>(0.22, 0.28, 0.38) * global_flash;

        // Slate rim.
        color = mix(color, vec3<f32>(0.30, 0.38, 0.52), rim * 0.65);

        alpha = (0.22 + field * 0.08 + arcs * 0.45 + global_flash * 0.25) * edge_fade
              + rim * 0.30;
    } else {
        // ---- Gravitational ----
        // Concentric rings that contract toward the center over time —
        // they look like spacetime collapsing into the void. The rings
        // are squeezed harder near the center (lens distortion) and
        // the swirl rotates the angle by distance so they don't look
        // perfectly concentric. Pure black hole in the middle.
        // ---- Gravitational ----
        // A heavy void cloud, no rings, no bullseye. Two layers of
        // fbm sampled with rotational shear so the cloud slowly
        // *swirls* without forming concentric rings. The center
        // darkens into a faint dark patch (not a hard black hole).
        // Reads as a drained, wrong-feeling patch of ground.
        //
        // Rotational shear: rotate the world coords by an angle that
        // varies with distance and time. This is what makes the
        // cloud look like it's being twisted, without producing the
        // geometric rings the old version had.
        let shear_angle = t * 0.12 + dist * 1.4;
        let cs = cos(shear_angle);
        let sn = sin(shear_angle);
        let sheared = vec2<f32>(
            world.x * cs - world.y * sn,
            world.x * sn + world.y * cs,
        );

        let cloud_a = fbm(sheared * 0.012 + vec2<f32>(t * 0.05, t * -0.03), 5);
        let cloud_b = fbm(sheared * 0.025 - vec2<f32>(t * 0.04, 0.0), 4);
        let cloud = cloud_a * 0.65 + cloud_b * 0.35;

        // Soft dark center, no hard edge.
        let center_dim = 1.0 - smoothstep(0.0, 0.55, dist);

        // Desaturated bruise palette.
        let base = vec3<f32>(0.07, 0.06, 0.10);     // deep slate
        let mid_col = vec3<f32>(0.16, 0.13, 0.20);  // faded plum
        let highlight = vec3<f32>(0.26, 0.22, 0.32);// dim lavender
        color = mix(base, mid_col, cloud);
        color = mix(color, highlight, smoothstep(0.55, 0.85, cloud) * 0.5);
        // Darken toward the center — not a black hole, just a sink.
        color *= 1.0 - center_dim * 0.55;

        // Muted plum rim.
        color = mix(color, vec3<f32>(0.22, 0.16, 0.28), rim * 0.6);

        alpha = (0.22 + cloud * 0.18 + center_dim * 0.15) * edge_fade
              + rim * 0.26;
    }

    // Intensity controls overall vividness. Tuned subtle — even
    // high-tier anomalies sit on the ground, they don't shout.
    alpha *= 0.28 + params.intensity * 0.45;

    if alpha < 0.005 {
        discard;
    }

    return vec4<f32>(color, clamp(alpha, 0.0, 1.0));
}
