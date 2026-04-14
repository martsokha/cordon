//! Combat resolution: weapon firing, damage application, hostility
//! checks.
//!
//! Engagement *decisions* (which target, when to advance) live in
//! [`crate::behavior::squad`]. This subplugin owns the per-NPC firing loop:
//! reading the [`CombatTarget`] component the squad system wrote,
//! ticking [`FireState`] cooldowns, applying damage when ready, and
//! emitting [`ShotFired`] events for the visual layer to render.

pub mod components;
pub mod events;
pub mod helpers;
pub mod systems;

use bevy::prelude::*;
pub use components::{CombatTarget, FireState};
pub use events::{NpcPoolChanged, ShotFired};
pub use helpers::{is_hostile, line_blocked, weapon_range};

use crate::plugin::SimSet;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShotFired>();
        app.add_message::<NpcPoolChanged>();
        app.add_systems(Update, systems::resolve_combat.in_set(SimSet::Combat));
    }
}
