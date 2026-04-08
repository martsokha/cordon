//! Visual reactions to sim events.
//!
//! All NPC simulation (movement, combat, squad AI, looting, death)
//! lives in `cordon-sim` and runs inside `SimSet`. This module
//! holds the *visual* systems that subscribe to sim events:
//!
//! - tracer rendering on `ShotFired`
//! - dot → X swap on `NpcDied`
//!
//! Other visual touch-ups (NPC dot mesh attach on spawn) live
//! alongside their concerns in `laptop::npcs` and `laptop::map`.

pub mod combat;
pub mod death;

use bevy::prelude::*;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((combat::CombatVisualsPlugin, death::DeathVisualsPlugin));
    }
}
