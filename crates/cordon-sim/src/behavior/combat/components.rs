//! Per-NPC combat state.

use bevy::prelude::*;

/// The hostile NPC this entity is firing on. `None` means the NPC is
/// not currently engaged in combat. The squad engagement scanner sets
/// this; the combat firing system reads it.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct CombatTarget(pub Option<Entity>);

/// Per-NPC firing state: cooldown until next shot.
///
/// Reload is not modelled as a timed phase — magazines refill
/// instantly from the general pouch when empty, and fire tempo is
/// controlled entirely by the weapon's `fire_rate`.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct FireState {
    pub cooldown_secs: f32,
}
