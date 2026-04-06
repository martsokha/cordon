//! Procedural terrain, cloud, anomaly, and CRT rendering.

mod anomaly;
mod clouds;
mod crt;
mod daynight;
mod terrain;

use bevy::prelude::*;

use crate::PlayingState;

#[derive(Resource)]
struct EnvironmentSpawned;

pub use self::crt::CrtEnabled;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            terrain::TerrainPlugin,
            clouds::CloudPlugin,
            anomaly::AnomalyPlugin,
            crt::CrtPlugin,
        ));
        app.add_systems(
            OnEnter(PlayingState::Laptop),
            spawn_environment.run_if(not(resource_exists::<EnvironmentSpawned>)),
        );
        app.add_systems(
            Update,
            daynight::sync_day_night.run_if(in_state(PlayingState::Laptop)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_environment(
    mut commands: Commands,
    game_data: Res<cordon_data::gamedata::GameDataResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut terrain_mats: ResMut<Assets<terrain::TerrainMaterial>>,
    mut cloud_mats: ResMut<Assets<clouds::CloudMaterial>>,
    mut anomaly_mats: ResMut<Assets<anomaly::AnomalyMaterial>>,
    mut crt_mats: ResMut<Assets<crt::CrtMaterial>>,
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50000.0, 50000.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
        Transform::from_xyz(0.0, 0.0, 0.0001),
    ));

    terrain::spawn(&mut commands, &mut meshes, &mut terrain_mats);
    anomaly::spawn(&mut commands, &game_data, &mut meshes, &mut anomaly_mats);
    clouds::spawn(&mut commands, &mut meshes, &mut cloud_mats);
    crt::spawn(&mut commands, &mut meshes, &mut crt_mats);

    commands.insert_resource(EnvironmentSpawned);
}
