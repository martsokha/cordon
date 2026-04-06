//! Procedural terrain and cloud rendering.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::PlayingState;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<TerrainMaterial>::default(),
            Material2dPlugin::<CloudMaterial>::default(),
        ));
        app.add_systems(OnEnter(PlayingState::Laptop), spawn_environment);
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CloudMaterial {}

impl Material2d for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/clouds.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn spawn_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
) {
    // Black background that extends far beyond the terrain
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50000.0, 50000.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Transform::from_xyz(0.0, 0.0, 0.0001),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(5000.0, 5000.0))),
        MeshMaterial2d(terrain_mats.add(TerrainMaterial {})),
        Transform::from_xyz(0.0, 0.0, 0.001),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(12000.0, 12000.0))),
        MeshMaterial2d(cloud_mats.add(CloudMaterial {})),
        Transform::from_xyz(0.0, 0.0, 5.0),
    ));
}
