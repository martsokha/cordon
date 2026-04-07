//! Visual reactions to sim events.
//!
//! All NPC simulation (movement, combat, squad AI, looting, death) now
//! lives in `cordon-sim` and runs inside `SimSet`. This module only
//! holds the **visual** systems that subscribe to sim events:
//!
//! - tracer rendering on `ShotFired`
//! - dot → X swap on `NpcDied`
//!
//! Other visual touch-ups (NPC dot mesh attach on spawn) live in the
//! laptop module.

pub mod combat;
pub mod death;

use bevy::prelude::*;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((combat::CombatVisualsPlugin, death::DeathVisualsPlugin));
    }
}
