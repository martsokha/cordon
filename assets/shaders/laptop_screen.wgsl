// Laptop screen — samples the render-target image fed by the
// laptop UI camera and draws it unfiltered on the laptop's screen
// face. No CRT effects; the laptop reads as a modern LCD.

#import bevy_pbr::forward_io::VertexOutput

@group(3) @binding(0) var feed_tex: texture_2d<f32>;
@group(3) @binding(1) var feed_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(feed_tex, feed_sampler, in.uv).rgb;
    return vec4<f32>(color, 1.0);
}
