//! `LaptopMaterial`: a thin shader material that samples the
//! laptop-UI render-target image and draws it on a 3D mesh in the
//! bunker (the laptop's screen face). No CRT filter — the laptop
//! reads as a modern LCD, not a security monitor.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LaptopMaterial {
    /// The image rendered by the laptop UI camera. Bound as a
    /// 2D texture + sampler so the WGSL shader can read it.
    #[texture(0)]
    #[sampler(1)]
    pub feed: Handle<Image>,
}

impl LaptopMaterial {
    pub fn new(feed: Handle<Image>) -> Self {
        Self { feed }
    }
}

impl Material for LaptopMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/laptop_screen.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
