//! Per-NPC looting state.

use bevy::prelude::*;

/// Per-NPC looting progress. Present only while the NPC is actively
/// looting a specific corpse. Removed when the corpse is empty or the
/// NPC walks away.
#[derive(Component, Debug, Clone, Copy)]
pub struct LootState {
    pub corpse: Entity,
    pub progress_secs: f32,
}
