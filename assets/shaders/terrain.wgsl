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

const PIXEL_SIZE: f32 = 1.0;

fn pixelate(p: vec2<f32>) -> vec2<f32> {
    return floor(p / PIXEL_SIZE) * PIXEL_SIZE;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let snapped = pixelate(in.world_position.xy);
    let uv = snapped * 0.002;

    // Elevation: large-scale height map
    let elevation = fbm(uv * 0.8, 6);
    // Moisture: determines biome type
    let moisture = fbm(uv * 0.6 + 50.0, 5);
    // Detail noise for texture
    let detail = fbm(uv * 4.0 + 13.0, 4);

    let deep_forest = vec3<f32>(0.06, 0.10, 0.04);
    let forest = vec3<f32>(0.10, 0.15, 0.06);
    let grassland = vec3<f32>(0.14, 0.16, 0.08);
    let swamp = vec3<f32>(0.06, 0.08, 0.05);
    let rocky = vec3<f32>(0.12, 0.11, 0.09);
    let dirt_road = vec3<f32>(0.16, 0.13, 0.09);

    // Biome selection
    var color: vec3<f32>;
    if elevation < 0.35 {
        // Low elevation: swamp/water areas
        color = mix(swamp, grassland, smoothstep(0.2, 0.35, elevation));
    } else if elevation < 0.55 {
        // Mid elevation: forest or grassland based on moisture
        color = mix(grassland, forest, smoothstep(0.4, 0.7, moisture));
    } else if elevation < 0.7 {
        // Higher: dense forest or rocky
        color = mix(forest, deep_forest, smoothstep(0.55, 0.7, elevation));
        color = mix(color, rocky, smoothstep(0.6, 0.8, 1.0 - moisture));
    } else {
        // High: rocky terrain
        color = mix(rocky, dirt_road, detail * 0.3);
    }

    // Detail texture variation
    color += (detail - 0.5) * 0.015;

    // Contour lines (topographic)
    let contour_interval = 0.08;
    let contour_val = fract(elevation / contour_interval);
    let contour_line = 1.0 - smoothstep(0.02, 0.05, min(contour_val, 1.0 - contour_val));
    color = mix(color, color * 0.6, contour_line * 0.4);

    // Coordinate grid (faint)
    let grid_size = 200.0;
    let grid = snapped / grid_size;
    let grid_line_x = 1.0 - smoothstep(0.0, 2.0 / grid_size, abs(fract(grid.x) - 0.5) * 2.0);
    let grid_line_y = 1.0 - smoothstep(0.0, 2.0 / grid_size, abs(fract(grid.y) - 0.5) * 2.0);
    let grid_line = max(grid_line_x, grid_line_y);
    color = mix(color, vec3<f32>(0.08, 0.08, 0.06), grid_line * 0.15);

    // Roads: procedural paths using warped noise
    let road_uv = snapped * 0.0008;
    let road_warp = fbm(road_uv * 3.0 + 7.0, 3) * 0.3;
    let road_val = abs(noise(road_uv + vec2<f32>(road_warp, 0.0)) - 0.5);
    let road = 1.0 - smoothstep(0.01, 0.04, road_val);
    color = mix(color, dirt_road, road * 0.5);

    // Fade to black at edges
    let world = in.world_position.xy;
    let edge = max(abs(world.x), abs(world.y));
    let fade = 1.0 - smoothstep(2000.0, 2500.0, edge);
    color *= fade;

    return vec4<f32>(color, 1.0);
}
