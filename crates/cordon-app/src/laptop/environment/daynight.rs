//! Day/night cycle synchronization for terrain and cloud materials.

use bevy::prelude::*;
use cordon_sim::resources::GameClock;

use super::clouds::{CloudEntity, CloudMaterial};
use super::terrain::{TerrainEntity, TerrainMaterial};

pub fn sync_day_night(
    clock: Option<Res<GameClock>>,
    terrain_q: Query<&MeshMaterial2d<TerrainMaterial>, With<TerrainEntity>>,
    cloud_q: Query<&MeshMaterial2d<CloudMaterial>, With<CloudEntity>>,
    mut terrain_mats: ResMut<Assets<TerrainMaterial>>,
    mut cloud_mats: ResMut<Assets<CloudMaterial>>,
) {
    let Some(clock) = clock else { return };
    let progress = clock.0.day_progress();

    // Change-guarded: `Assets<T>::get_mut` marks the asset as
    // changed and re-uploads its uniform buffer to the GPU every
    // frame we touch it. The day/night progress changes very
    // slowly (seconds of in-game time per real frame), so we only
    // need to touch the materials when the float actually shifts
    // past a perceptible threshold.
    const PROGRESS_EPS: f32 = 1.0 / 1024.0;
    for handle in &terrain_q {
        let needs_update = terrain_mats
            .get(&handle.0)
            .map(|m| (m.day_progress - progress).abs() > PROGRESS_EPS)
            .unwrap_or(false);
        if needs_update && let Some(mat) = terrain_mats.get_mut(&handle.0) {
            mat.day_progress = progress;
        }
    }
    for handle in &cloud_q {
        let needs_update = cloud_mats
            .get(&handle.0)
            .map(|m| (m.day_progress - progress).abs() > PROGRESS_EPS)
            .unwrap_or(false);
        if needs_update && let Some(mat) = cloud_mats.get_mut(&handle.0) {
            mat.day_progress = progress;
        }
    }
}
