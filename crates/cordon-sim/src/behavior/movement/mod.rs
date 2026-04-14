//! Per-NPC movement: [`MovementTarget`] + [`MovementSpeed`] components
//! and the `move_npcs` system that walks each NPC toward its target.

pub mod components;
pub mod constants;
pub mod systems;

use bevy::prelude::*;
pub use components::{MovementSpeed, MovementTarget};
pub use systems::move_npcs;

use crate::plugin::SimSet;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_npcs.in_set(SimSet::Movement));
    }
}
