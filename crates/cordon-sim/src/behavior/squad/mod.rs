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
//! - [`identity`]   — SquadMarker / SquadLeader / SquadMembers /
//!   SquadMembership / SquadBundle. Pure ECS data, no systems.
//! - [`intent`]     — the two blackboard components (MovementIntent,
//!   EngagementTarget) that the deciders (behave, engagement) write
//!   and the mover (formation) reads.
//! - [`constants`]  — timing / distance tuning knobs.
//! - [`commands`]   — player → sim command boundary.
//! - [`lifecycle`]  — drop despawned members, promote leaders, prune
//!   stale membership back-pointers.
//! - [`formation`]  — SquadFacing / SquadWaypoints / SquadHomePosition
//!   components plus the system that turns MovementIntent into per-
//!   member MovementTarget.
//! - [`engagement`] — vision-shared hostile selection + per-member
//!   combat targets. Writes EngagementTarget. Uses [`scan`] internally.
//! - [`scan`]       — spatial grid + per-NPC snapshot, an internal
//!   helper for engagement.
//! - [`behave`]     — behavior-tree attach observer, action leaves,
//!   and the Goal → Tree factory. Writes MovementIntent.

pub mod behave;
pub mod commands;
pub mod constants;
mod engagement;
pub mod formation;
pub mod identity;
pub mod intent;
mod lifecycle;
mod scan;

use bevy::prelude::*;
use bevy_behave::prelude::BehavePlugin;
pub use commands::{Owned, SquadCommand};
pub use formation::{SquadFacing, SquadHomePosition, SquadWaypoints};
pub use identity::{SquadBundle, SquadLeader, SquadMarker, SquadMembers, SquadMembership};
pub use intent::{EngagementTarget, MovementIntent};

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
