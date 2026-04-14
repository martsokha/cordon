//! Squad behavior: vision-shared engagement, formation positioning,
//! goal transitions (via behavior trees), lifecycle, and the player
//! command boundary.
//!
//! Squads are Bevy entities. NPC members carry a `SquadMembership`
//! back-pointer. Hot-path systems iterate squads + members via ECS
//! queries — there is no HashMap fallback.
//!
//! Submodules:
//!
//! - [`components`] — SquadMarker/Leader/Members/Facing/Bundle, plus
//!                    MovementIntent / EngagementTarget (the two
//!                    BT-written data components)
//! - [`constants`]  — timing / distance tuning knobs
//! - [`scan`]       — spatial grid + per-NPC snapshot used by engagement
//! - [`engagement`] — vision-shared hostile selection + per-member targets
//! - [`formation`]  — formation slot positioning
//! - [`lifecycle`]  — drop despawned members, promote leaders, prune stale membership
//! - [`commands`]   — player → sim command boundary (the only mutation path)
//! - [`behave`]     — behavior-tree attach observer + action leaves + tree factory

pub mod behave;
pub mod commands;
pub mod components;
pub mod constants;
mod engagement;
mod formation;
mod lifecycle;
mod scan;

use bevy::prelude::*;
use bevy_behave::prelude::BehavePlugin;

pub use commands::{Owned, SquadCommand};
pub use components::{
    EngagementTarget, MovementIntent, SquadBundle, SquadFacing, SquadHomePosition, SquadLeader,
    SquadMarker, SquadMembers, SquadMembership, SquadWaypoints,
};

use crate::plugin::SimSet;

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BehavePlugin::default());
        app.add_plugins(behave::ActionsPlugin);
        app.add_message::<SquadCommand>();
        app.add_observer(behave::attach_goal_tree);
        app.add_systems(
            Update,
            (
                commands::apply_squad_commands.in_set(SimSet::Commands),
                (
                    lifecycle::prune_stale_membership,
                    lifecycle::cleanup_dead_squads,
                )
                    .in_set(SimSet::Cleanup),
                engagement::update_squad_engagement.in_set(SimSet::Engagement),
                formation::drive_squad_formation.in_set(SimSet::Formation),
            ),
        );
    }
}
