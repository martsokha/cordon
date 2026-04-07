//! NPC behavior: per-NPC `Action` state machine.
//!
//! Squads decide *what* a member should be doing through their goal +
//! activity (see [`super::squad`]); this module just provides the
//! per-NPC physical state (walk, idle, fire, loot) and a tiny driver
//! that advances Walk/Idle/Flee transforms each tick.

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::Uid;
use moonshine_behavior::prelude::*;

use super::death::Dead;

/// What the NPC is physically doing right now.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub enum Action {
    /// Standing around, waiting.
    Idle { timer: f32 },
    /// Moving toward a point.
    Walk { target: Vec2, speed: f32 },
    /// Following another NPC.
    Follow {
        #[reflect(ignore)]
        target: Uid<Npc>,
    },
    /// At the counter, engaged in trade. Timer counts down.
    Trade { timer: f32 },
    /// Running from danger.
    Flee { target: Vec2 },
    /// Firing on a target NPC. `cooldown_secs` ticks down toward 0; on
    /// reaching 0 the combat system applies one shot's damage and
    /// resets the cooldown to the weapon's interval. While
    /// `reload_secs > 0` the NPC is reloading and cannot fire.
    Engage {
        #[reflect(ignore)]
        target: Uid<Npc>,
        cooldown_secs: f32,
        reload_secs: f32,
    },
    /// Looting a corpse. `progress_secs` ticks down toward 0; on
    /// reaching 0 the loot system transfers one item and resets.
    Loot {
        #[reflect(ignore)]
        target: Uid<Npc>,
        progress_secs: f32,
    },
}

impl Behavior for Action {
    fn filter_next(&self, next: &Self) -> bool {
        use Action::*;
        match_next! {
            self => next,
            Idle { .. } => Walk { .. } | Trade { .. } | Follow { .. } | Flee { .. } | Engage { .. } | Loot { .. },
            Walk { .. } => Idle { .. } | Trade { .. } | Follow { .. } | Flee { .. } | Engage { .. } | Loot { .. },
            Follow { .. } => Idle { .. } | Walk { .. } | Flee { .. } | Engage { .. },
            Trade { .. } => Idle { .. } | Walk { .. } | Flee { .. } | Engage { .. },
            Flee { .. } => Idle { .. } | Walk { .. },
            Engage { .. } => Idle { .. } | Walk { .. } | Flee { .. } | Engage { .. },
            Loot { .. } => Idle { .. } | Walk { .. } | Flee { .. } | Engage { .. }
        }
    }
}

/// Half-extent of the playable map AABB. NPC positions are clamped to
/// `±MAP_BOUND` so they can't walk off the world during combat or
/// formation moves.
pub const MAP_BOUND: f32 = 1500.0;

/// Drive NPC actions based on their current state. Dead NPCs are
/// excluded so corpses don't keep walking, fleeing, or wandering.
#[allow(clippy::type_complexity)]
pub fn drive_actions(
    time: Res<Time>,
    mut query: Query<(BehaviorMut<Action>, &mut Transform), Without<Dead>>,
) {
    let dt = time.delta_secs();

    for (mut behavior, mut transform) in &mut query {
        let pos = transform.translation.truncate();

        match behavior.current_mut() {
            Action::Idle { timer } => {
                // Tick the timer; do not wander. Wander would push the
                // NPC out of its formation slot, causing the squad
                // system to push another Walk and visually flicker.
                *timer -= dt;
            }
            Action::Walk { target, speed } => {
                let dir = (*target - pos).normalize_or_zero();
                transform.translation.x += dir.x * *speed * dt;
                transform.translation.y += dir.y * *speed * dt;
            }
            Action::Follow { .. } => {}
            Action::Trade { timer } => {
                *timer -= dt;
            }
            Action::Flee { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 20.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;
            }
            Action::Engage { .. } => {
                // Engage logic (cooldown, damage) lives in the combat system.
            }
            Action::Loot { .. } => {
                // Loot logic lives in the loot system.
            }
        }

        // Keep NPCs inside the playable map. Walking NPCs will keep
        // pushing against the boundary harmlessly until their action
        // changes; the squad system shouldn't pick targets outside
        // bounds in the first place.
        transform.translation.x = transform.translation.x.clamp(-MAP_BOUND, MAP_BOUND);
        transform.translation.y = transform.translation.y.clamp(-MAP_BOUND, MAP_BOUND);
    }
}
