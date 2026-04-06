//! Procedural terrain, cloud, anomaly, and CRT rendering.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use cordon_core::primitive::Tier;
use cordon_data::gamedata::GameDataResource;

use crate::PlayingState;
use crate::world::SimWorld;

/// Whether the CRT effect is active (disabled by laptop upgrade).
#[derive(Resource)]
pub struct CrtEnabled(pub bool);

impl Default for CrtEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Marker for the CRT overlay entity.
#[derive(Component)]
struct CrtOverlay;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrtEnabled>();
        app.add_plugins((
            Material2dPlugin::<TerrainMaterial>::default(),
            Material2dPlugin::<CloudMaterial>::default(),
            Material2dPlugin::<AnomalyMaterial>::default(),
            Material2dPlugin::<CrtMaterial>::default(),
        ));
        app.add_systems(OnEnter(PlayingState::Laptop), spawn_environment);
        app.add_systems(
            Update,
            (toggle_crt, sync_day_night).run_if(in_state(PlayingState::Laptop)),
        );
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    /// 0.0 = midnight, 0.5 = noon, 1.0 = midnight.
    #[uniform(0)]
    pub day_progress: f32,
}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CloudMaterial {
    /// 0.0 = midnight, 0.5 = noon, 1.0 = midnight.
    #[uniform(0)]
    pub day_progress: f32,
}

impl Material2d for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/clouds.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
struct TerrainEntity;

#[derive(Component)]
struct CloudEntity;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct AnomalyMaterial {
    #[uniform(0)]
    pub hazard_type: f32,
    #[uniform(0)]
    pub intensity: f32,
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

fn tier_to_intensity(t: &Tier) -> f32 {
    match t {
        Tier::VeryLow => 0.2,
        Tier::Low => 0.4,
        Tier::Medium => 0.6,
        Tier::High => 0.8,
        Tier::VeryHigh => 1.0,
    }
}

fn hazard_type_to_float(h: &cordon_core::primitive::HazardType) -> f32 {
    match h {
        cordon_core::primitive::HazardType::Chemical => 0.0,
        cordon_core::primitive::HazardType::Thermal => 1.0,
        cordon_core::primitive::HazardType::Electric => 2.0,
        cordon_core::primitive::HazardType::Gravitational => 3.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_environment(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
    mut anomaly_mats: ResMut<Assets<AnomalyMaterial>>,
    mut crt_mats: ResMut<Assets<CrtMaterial>>,
) {
    // Black background
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50000.0, 50000.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Transform::from_xyz(0.0, 0.0, 0.0001),
    ));

    // Terrain
    commands.spawn((
        TerrainEntity,
        Mesh2d(meshes.add(Rectangle::new(5000.0, 5000.0))),
        MeshMaterial2d(terrain_mats.add(TerrainMaterial { day_progress: 0.33 })),
        Transform::from_xyz(0.0, 0.0, 0.001),
    ));

    // Anomaly zones — one per hazardous area
    for area in game_data.0.areas.values() {
        if let Some(hazard) = &area.danger.hazard {
            let x = area.location.x;
            let y = area.location.y;
            let r = area.radius.value() * 1.2;
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(r))),
                MeshMaterial2d(anomaly_mats.add(AnomalyMaterial {
                    hazard_type: hazard_type_to_float(&hazard.kind),
                    intensity: tier_to_intensity(&hazard.intensity),
                    _padding1: 0.0,
                    _padding2: 0.0,
                })),
                Transform::from_xyz(x, y, 3.0),
            ));
        }
    }

    // Clouds
    commands.spawn((
        CloudEntity,
        Mesh2d(meshes.add(Rectangle::new(5000.0, 5000.0))),
        MeshMaterial2d(cloud_mats.add(CloudMaterial { day_progress: 0.33 })),
        Transform::from_xyz(0.0, 0.0, 5.0),
    ));

    // CRT overlay (huge quad that covers the full camera view, always in front)
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

fn sync_day_night(
    sim: Option<Res<SimWorld>>,
    terrain_q: Query<&MeshMaterial2d<TerrainMaterial>, With<TerrainEntity>>,
    cloud_q: Query<&MeshMaterial2d<CloudMaterial>, With<CloudEntity>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
) {
    let Some(sim) = sim else { return };
    let progress = sim.0.time.day_progress();

    for handle in &terrain_q {
        if let Some(mat) = terrain_mats.get_mut(&handle.0) {
            mat.day_progress = progress;
        }
    }
    for handle in &cloud_q {
        if let Some(mat) = cloud_mats.get_mut(&handle.0) {
            mat.day_progress = progress;
        }
    }
}
