//! Bevy plugin entry point for the world simulation.
//!
//! [`CordonSimPlugin`] is the public face of `cordon-sim`. The
//! game-side `cordon-bevy` crate adds it to the Bevy app and the
//! plugin takes care of:
//!
//! - Registering NPC and squad components
//! - Running [`crate::spawn::spawn_population`] to keep the alive
//!   population at the target value
//!
//! All cordon-sim systems run inside the [`SimSet`] schedule so
//! downstream crates (cordon-bevy AI, laptop visuals) can declare
//! `.after(SimSet::Spawn)` and similar without naming individual
//! function symbols.

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use crate::resources::{SimWorld, SquadIdIndex};
use crate::spawn;

/// Ordered system set for cordon-sim's Bevy systems. Downstream
/// crates use this for explicit ordering of their own systems
/// relative to sim updates.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Population top-up: spawns NPC and squad entities.
    Spawn,
}

/// Public Bevy plugin for the cordon world simulation.
pub struct CordonSimPlugin;

impl Plugin for CordonSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadIdIndex>();
        app.configure_sets(
            Update,
            SimSet::Spawn
                .run_if(resource_exists::<SimWorld>)
                .run_if(resource_exists::<GameDataResource>),
        );
        app.add_systems(Update, spawn::spawn_population.in_set(SimSet::Spawn));
    }
}

/// Re-exports for convenience.
pub mod prelude {
    pub use super::{CordonSimPlugin, SimSet};
    pub use crate::components::{
        Employment, FactionId, Hp, LoadoutComp, Loyalty, NpcBundle, NpcId, NpcMarker, NpcNameComp,
        PersonalityComp, Perks, SquadActivity, SquadBundle, SquadFacing, SquadFaction,
        SquadFormation, SquadGoal, SquadHomePosition, SquadId, SquadLeader, SquadMarker,
        SquadMembers, SquadMembership, SquadWaypoints, Trust, Wealth, Xp,
    };
    pub use crate::resources::{SimWorld, SquadIdIndex};
}
