//! Per-NPC physical and combat state, plus the per-NPC movement system.
//!
//! These components are written by the squad/combat/loot systems and
//! read by the per-NPC drivers below. There is no state machine —
//! interruptions are just "set a new target, drop the old one", which
//! is naturally race-free under explicit system ordering.

use bevy::prelude::*;
use cordon_core::primitive::{GameTime, Rank};

use crate::plugin::SimSet;

/// Half-extent of the playable map AABB. NPC positions are clamped to
/// `±MAP_BOUND` so they can't walk off the world during combat or
/// formation moves.
pub const MAP_BOUND: f32 = 1500.0;

/// The point this NPC is currently walking toward, in world space.
/// `None` means the NPC is standing still.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct MovementTarget(pub Option<Vec2>);

/// How fast this NPC walks toward [`MovementTarget`], in map units per
/// second. Updated by the system that sets the target.
#[derive(Component, Debug, Clone, Copy)]
pub struct MovementSpeed(pub f32);

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(30.0)
    }
}

/// The hostile NPC this entity is firing on. `None` means the NPC is
/// not currently engaged in combat. The squad engagement scanner sets
/// this; the combat firing system reads it.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct CombatTarget(pub Option<Entity>);

/// Per-NPC firing state: cooldown until next shot and reload progress.
/// While `reload_secs > 0`, no shots fire.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct FireState {
    pub cooldown_secs: f32,
    pub reload_secs: f32,
}

/// Per-NPC looting progress. Present only while the NPC is actively
/// looting a specific corpse. Removed when the corpse is empty or the
/// NPC walks away.
#[derive(Component, Debug, Clone, Copy)]
pub struct LootState {
    pub corpse: Entity,
    pub progress_secs: f32,
}

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 120 base + 15 per rank tier above Novice + 25 if
    /// the NPC's faction has military training.
    pub fn for_npc(rank: Rank, is_military: bool) -> Self {
        let from_rank = 120.0 + (rank.tier() as f32 - 1.0) * 15.0;
        let from_faction = if is_military { 25.0 } else { 0.0 };
        Self {
            radius: from_rank + from_faction,
        }
    }
}

/// Marker for anomaly entities, contributing to LOS blocking. Spawned
/// by the visual layer when it lays out the map; the combat system
/// reads `(Transform, AnomalyZone)` to compute LOS.
#[derive(Component, Debug, Clone, Copy)]
pub struct AnomalyZone {
    pub radius: f32,
}

/// Marker for a corpse with its time of death. Inserted by the death
/// system when an NPC's HP hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
}

/// Walk every NPC with a [`MovementTarget`] toward that point.
///
/// Filters on `MovementTarget` so NPCs that aren't moving don't even
/// touch their transform — Bevy's change detection skips them and the
/// downstream transform-propagation system has less work.
#[allow(clippy::type_complexity)]
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

/// Plugin registering the movement system in [`SimSet::Movement`].
pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_npcs.in_set(SimSet::Movement));
    }
}
