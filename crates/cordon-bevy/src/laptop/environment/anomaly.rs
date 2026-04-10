//! Anomaly zone shader material and spawning.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use cordon_core::primitive::Tier;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::behavior::AnomalyZone;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct AnomalyMaterial {
    #[uniform(0)]
    pub intensity: f32,
    #[uniform(0)]
    pub _padding0: f32,
    #[uniform(0)]
    pub _padding1: f32,
    #[uniform(0)]
    pub _padding2: f32,
}

impl Material2d for AnomalyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/anomaly.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct AnomalyPlugin;

impl Plugin for AnomalyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<AnomalyMaterial>::default());
    }
}

fn tier_to_intensity(t: &Tier) -> f32 {
    match t {
        Tier::VeryLow => 0.2,
        Tier::Low => 0.4,
        Tier::Medium => 0.6,
        Tier::High => 0.8,
        Tier::VeryHigh => 1.0,
    }
}

/// Marker for the 2D visual mesh of an anomaly zone. Distinct from
/// [`AnomalyZone`] (which is the sim-side LOS blocker) because the
/// fog-of-war system hides the *visual* without touching the
/// underlying sim component.
#[derive(Component)]
pub struct AnomalyVisual;

pub fn spawn(
    commands: &mut Commands,
    game_data: &GameDataResource,
    meshes: &mut ResMut<Assets<Mesh>>,
    anomaly_mats: &mut ResMut<Assets<AnomalyMaterial>>,
) {
    for area in game_data.0.areas.values() {
        if !area.kind.is_anomaly() {
            continue;
        }
        let Some(corruption) = area.kind.corruption() else {
            continue;
        };
        let x = area.location.x;
        let y = area.location.y;
        let r = area.radius.value() * 1.2;
        commands.spawn((
            AnomalyZone { radius: r },
            AnomalyVisual,
            Mesh2d(meshes.add(Circle::new(r))),
            MeshMaterial2d(anomaly_mats.add(AnomalyMaterial {
                intensity: tier_to_intensity(&corruption),
                _padding0: 0.0,
                _padding1: 0.0,
                _padding2: 0.0,
            })),
            // Above the fog overlay (z=4.5) and clouds (z=5.0) so
            // the shader effects render on top of the mist instead
            // of being darkened by it.
            Transform::from_xyz(x, y, 8.5),
        ));
    }
}
