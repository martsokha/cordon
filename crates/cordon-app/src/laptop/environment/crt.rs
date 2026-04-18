//! CRT overlay shader material and toggle.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::PlayingState;

#[derive(Resource)]
pub struct CrtEnabled(pub bool);

impl Default for CrtEnabled {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
struct CrtOverlay;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CrtMaterial {}

impl Material2d for CrtMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/crt.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct CrtPlugin;

impl Plugin for CrtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrtEnabled>();
        app.add_plugins(Material2dPlugin::<CrtMaterial>::default());
        app.add_systems(Update, toggle_crt.run_if(in_state(PlayingState::Laptop)));
    }
}

pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    crt_mats: &mut ResMut<Assets<CrtMaterial>>,
) {
    commands.spawn((
        CrtOverlay,
        Mesh2d(meshes.add(Rectangle::new(50000.0, 50000.0))),
        MeshMaterial2d(crt_mats.add(CrtMaterial {})),
        Transform::from_xyz(0.0, 0.0, 7.0),
    ));
}

fn toggle_crt(crt: Res<CrtEnabled>, mut query: Query<&mut Visibility, With<CrtOverlay>>) {
    if !crt.is_changed() {
        return;
    }
    for mut vis in &mut query {
        *vis = if crt.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
