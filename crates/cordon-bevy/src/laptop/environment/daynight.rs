//! Day/night cycle synchronization for terrain and cloud materials.

use bevy::prelude::*;

use super::clouds::{CloudEntity, CloudMaterial};
use super::terrain::{TerrainEntity, TerrainMaterial};
use crate::world::SimWorld;

pub fn sync_day_night(
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
