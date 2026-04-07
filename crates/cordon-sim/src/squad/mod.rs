//! Squad behavior: vision-shared engagement, formation positioning,
//! goal transitions, lifecycle, and the player command boundary.
//!
//! Squads are Bevy entities. NPC members carry a `SquadMembership`
//! back-pointer. Hot-path systems iterate squads + members via ECS
//! queries — there is no HashMap fallback.
//!
//! Submodules:
//!
//! - [`scan`]       — spatial grid + per-NPC snapshot used by engagement
//! - [`engagement`] — vision-shared hostile selection + per-member targets
//! - [`goals`]      — Hold timer expiry → next goal-driven activity
//! - [`formation`]  — formation slot positioning + Protect follow + arrival
//! - [`lifecycle`]  — drop dead members, promote leaders, despawn dead squads
//! - [`commands`]   — player → sim command boundary (the only mutation path)

mod commands;
mod engagement;
mod formation;
mod goals;
mod lifecycle;
mod scan;

use bevy::prelude::*;
pub use commands::{Owned, SquadCommand};

use crate::plugin::SimSet;

const SQUAD_WALK_SPEED: f32 = 30.0;
const ENGAGE_WALK_SPEED: f32 = 38.0;
const PATROL_HOLD_SECS: f32 = 6.0;
const ARRIVED_DIST: f32 = 12.0;

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SquadCommand>();
        app.add_systems(
            Update,
            (
                commands::apply_squad_commands.in_set(SimSet::Commands),
                lifecycle::cleanup_dead_squads.in_set(SimSet::Cleanup),
                goals::drive_squad_goals.in_set(SimSet::Goals),
                engagement::update_squad_engagement.in_set(SimSet::Engagement),
                formation::drive_squad_formation.in_set(SimSet::Formation),
            ),
        );
    }
}
