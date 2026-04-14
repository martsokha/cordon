//! Per-NPC movement systems.

use bevy::prelude::*;

use super::components::{MovementSpeed, MovementTarget};
use super::constants::MAP_BOUND;
use crate::behavior::death::components::Dead;

/// Walk every NPC with a [`MovementTarget`] toward that point.
///
/// Filters on `MovementTarget` so NPCs that aren't moving don't even
/// touch their transform — Bevy's change detection skips them and the
/// downstream transform-propagation system has less work.
pub fn move_npcs(
    time: Res<Time>,
    mut q: Query<(&MovementTarget, &MovementSpeed, &mut Transform), Without<Dead>>,
) {
    let dt = time.delta_secs();
    for (target, speed, mut transform) in &mut q {
        let Some(target) = target.0 else { continue };
        let pos = transform.translation.truncate();
        let delta = target - pos;
        let dist = delta.length();
        if dist < 0.5 {
            continue;
        }
        let dir = delta / dist;
        let step = (speed.0 * dt).min(dist);
        transform.translation.x += dir.x * step;
        transform.translation.y += dir.y * step;
        transform.translation.x = transform.translation.x.clamp(-MAP_BOUND, MAP_BOUND);
        transform.translation.y = transform.translation.y.clamp(-MAP_BOUND, MAP_BOUND);
    }
}
