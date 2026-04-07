//! Bevy plugin entry point for the world simulation.
//!
//! [`CordonSimPlugin`] is the public face of `cordon-sim`. It composes
//! every sub-plugin (spawn, behavior, squad AI, combat, death, loot)
//! and declares the [`SimSet`] order so cross-plugin systems run in a
//! well-defined sequence each frame.
//!
//! All cordon-sim systems run inside `SimSet`. Downstream crates
//! (cordon-bevy visuals, audio) can declare `.after(SimSet::X)` or
//! `.in_set(SimSet::X)` without naming individual function symbols.

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use crate::behavior::BehaviorPlugin;
use crate::combat::CombatPlugin;
use crate::death::DeathPlugin;
use crate::loot::LootPlugin;
use crate::resources::{SimWorld, SquadIdIndex};
use crate::spawn;
use crate::squad_ai::SquadAiPlugin;

/// Ordered system set for cordon-sim. The whole chain runs only when
/// both [`SimWorld`] and [`GameDataResource`] are present, so the sim
/// sleeps cleanly during loading.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Per-frame house-keeping (squad cleanup, etc).
    Cleanup,
    /// Population top-up: spawns NPC and squad entities.
    Spawn,
    /// Goal-driven activity transitions.
    Goals,
    /// Vision-shared engagement scan; writes per-NPC `CombatTarget`.
    Engagement,
    /// Per-NPC `MovementTarget` from formation slots.
    Formation,
    /// Apply `MovementTarget` to `Transform`.
    Movement,
    /// Fire weapons, apply damage, emit `ShotFired`.
    Combat,
    /// Tag dead NPCs, despawn corpses.
    Death,
    /// Adjacent looters pull items from corpses.
    Loot,
}

/// Public Bevy plugin for the cordon world simulation.
pub struct CordonSimPlugin;

impl Plugin for CordonSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadIdIndex>();

        app.configure_sets(
            Update,
            (
                SimSet::Cleanup,
                SimSet::Spawn,
                SimSet::Goals,
                SimSet::Engagement,
                SimSet::Formation,
                SimSet::Movement,
                SimSet::Combat,
                SimSet::Death,
                SimSet::Loot,
            )
                .chain()
                .run_if(resource_exists::<SimWorld>)
                .run_if(resource_exists::<GameDataResource>),
        );

        app.add_systems(Update, spawn::spawn_population.in_set(SimSet::Spawn));
        app.add_plugins((
            BehaviorPlugin,
            SquadAiPlugin,
            CombatPlugin,
            DeathPlugin,
            LootPlugin,
        ));
    }
}

/// Re-exports for convenience.
pub mod prelude {
    pub use super::{CordonSimPlugin, SimSet};
    pub use crate::behavior::{
        AnomalyZone, CombatTarget, Dead, FireState, LootState, MovementSpeed, MovementTarget,
        Vision, MAP_BOUND,
    };
    pub use crate::components::{
        Employment, FactionId, Hp, LoadoutComp, Loyalty, NpcBundle, NpcId, NpcMarker, NpcNameComp,
        PersonalityComp, Perks, SquadActivity, SquadBundle, SquadFacing, SquadFaction,
        SquadFormation, SquadGoal, SquadHomePosition, SquadId, SquadLeader, SquadMarker,
        SquadMembers, SquadMembership, SquadWaypoints, Trust, Wealth, Xp,
    };
    pub use crate::events::{
        CorpseRemoved, DayRolled, ItemLooted, NpcDied, ShotFired, SquadSpawned,
    };
    pub use crate::resources::{SimWorld, SquadIdIndex};
}
