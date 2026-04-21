//! `CctvMaterial`: a tiny shader material that takes the CCTV
//! render-target image and applies the "old security monitor" look
//! — scanlines, slight grain, vignette, edge bow.
//!
//! Bevy 0.18's `AsBindGroup` derive expects a uniform buffer at
//! binding 0 of group 2 (the material's bind group). We give it a
//! tiny params uniform (currently just a brightness multiplier,
//! which doubles as a placeholder for future per-instance shader
//! knobs) and put the texture + sampler on bindings 1 and 2.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CctvMaterial {
    /// How strongly to apply the CRT-style effects (scanlines,
    /// grain, vignette, tear, hum, static, phosphor tint). 1.0 is
    /// tuned for the small corner monitor; lower values (≈0.4) are
    /// used on the fullscreen plane where each effect covers the
    /// whole screen and would otherwise be overwhelming.
    ///
    /// Packed into a vec4-sized uniform: WGSL requires uniform
    /// buffers be 16-byte aligned, so we pad with three unused
    /// floats that all share binding 0. A bare `f32` would emit a
    /// 4-byte buffer and wgpu rejects it as a storage-class mismatch.
    #[uniform(0)]
    pub effect_strength: f32,
    #[uniform(0)]
    pub _pad1: f32,
    #[uniform(0)]
    pub _pad2: f32,
    #[uniform(0)]
    pub _pad3: f32,
    /// The image rendered by the CCTV camera. Bound as a 2D
    /// texture + sampler so the WGSL shader can read it.
    #[texture(1)]
    #[sampler(2)]
    pub feed: Handle<Image>,
}

impl CctvMaterial {
    pub fn new(feed: Handle<Image>, effect_strength: f32) -> Self {
        Self {
            effect_strength,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            feed,
        }
    }
}

impl Material for CctvMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cctv_screen.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
