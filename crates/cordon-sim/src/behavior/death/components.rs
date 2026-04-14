//! Per-NPC death marker component.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;

/// Marker for a corpse with its time of death. Inserted by the death
/// system when an NPC's HP hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
}
