//! Per-NPC physical state and the systems that drive it.
//!
//! Movement and combat targets are stored as plain ECS components
//! ([`MovementTarget`], [`CombatTarget`]) updated by the squad and
//! combat layers. Per-NPC drivers (`move_npcs`, `tick_idle_timers`)
//! read those components and advance the world. There is no state
//! machine — interruptions are just "set the new target, drop the
//! old one", which is naturally race-free under explicit system
//! ordering.

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::Uid;

use super::death::Dead;

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
pub struct CombatTarget(pub Option<Uid<Npc>>);

/// Per-NPC firing state: cooldown until next shot and reload progress.
/// Both timers tick toward zero in [`super::combat::resolve_combat`].
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
    pub corpse: Uid<Npc>,
    pub progress_secs: f32,
}

/// Walk every NPC with a [`MovementTarget`] toward that point.
///
/// Filters on `MovementTarget` so NPCs that aren't moving don't even
/// touch their transform — Bevy's change detection skips them and the
/// downstream transform-propagation system has less work.
#[allow(clippy::type_complexity)]
pub fn move_npcs(
    time: Res<Time>,
    mut q: Query<
        (&MovementTarget, &MovementSpeed, &mut Transform),
        Without<Dead>,
    >,
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
