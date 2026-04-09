//! Bevy plugin entry point for the world simulation.
//!
//! [`CordonSimPlugin`] is the public face of `cordon-sim`. It composes
//! every sub-plugin (day, behavior, squad, combat, death, loot) and
//! declares the [`SimSet`] order so cross-plugin systems run in a
//! well-defined sequence each frame.
//!
//! All cordon-sim systems run inside `SimSet`. Downstream crates
//! (cordon-bevy visuals, audio) can declare `.after(SimSet::X)` or
//! `.in_set(SimSet::X)` without naming individual function symbols.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::EntropyPlugin;
use cordon_data::gamedata::GameDataResource;

use crate::behavior::BehaviorPlugin;
use crate::combat::CombatPlugin;
use crate::day::DayCyclePlugin;
use crate::death::DeathPlugin;
use crate::effects::EffectsPlugin;
use crate::loot::LootPlugin;
use crate::quest::QuestPlugin;
use crate::resources::{GameClock, SquadIdIndex, UidAllocator};
use crate::spawn;
use crate::spawn::relics::RelicSpawnPlugin;
use crate::squad::SquadPlugin;

/// Ordered system set for cordon-sim. The whole chain runs only when
/// both [`GameClock`] and [`GameDataResource`] are present, so the
/// sim sleeps cleanly during loading.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Player commands applied first so an order issued this frame
    /// takes effect before any AI re-evaluation later in the same
    /// frame. The only place player intent enters the sim.
    Commands,
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
    /// Fire weapons, apply damage, emit `ShotFired` and
    /// `NpcDamaged`.
    Combat,
    /// Effect dispatch and active-effect ticking. Runs after
    /// combat so it can react to `NpcDamaged` messages, and
    /// before death handling so an `OnHpLow` heal can pull a
    /// carrier back from zero HP in the same frame.
    Effects,
    /// Tag dead NPCs, despawn corpses.
    Death,
    /// Adjacent looters pull items from corpses.
    Loot,
}

/// Public Bevy plugin for the cordon world simulation.
pub struct CordonSimPlugin;

impl Plugin for CordonSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.init_resource::<SquadIdIndex>();
        app.init_resource::<UidAllocator>();

        app.configure_sets(
            Update,
            (
                SimSet::Commands,
                SimSet::Cleanup,
                SimSet::Spawn,
                SimSet::Goals,
                SimSet::Engagement,
                SimSet::Formation,
                SimSet::Movement,
                SimSet::Combat,
                SimSet::Effects,
                SimSet::Death,
                SimSet::Loot,
            )
                .chain()
                .run_if(resource_exists::<GameClock>)
                .run_if(resource_exists::<GameDataResource>),
        );

        app.add_message::<spawn::SquadSpawned>();
        app.add_systems(Update, spawn::spawn_population.in_set(SimSet::Spawn));
        // Game clock ticks every frame once the world is
        // initialised. Gated on `GameClock` existing so it waits
        // for the cordon-bevy layer's `init_world_resources` call.
        app.add_systems(
            Update,
            crate::resources::tick_game_time.run_if(resource_exists::<GameClock>),
        );
        app.add_plugins((
            DayCyclePlugin,
            BehaviorPlugin,
            SquadPlugin,
            CombatPlugin,
            EffectsPlugin,
            DeathPlugin,
            LootPlugin,
            RelicSpawnPlugin,
            QuestPlugin,
        ));
    }
}

/// Re-exports for convenience.
pub mod prelude {
    // Cordon-core types that derive `Component` directly and are
    // attached to entities as live components, plus the flavour
    // types (`Trust`, `Loyalty`, `Personality`) that are bundled
    // inside `NpcAttributes`. Re-exported from the prelude so
    // consumers can pull "anything on an NPC entity" from one
    // place.
    pub use cordon_core::entity::name::NpcName;
    pub use cordon_core::entity::npc::Personality;
    pub use cordon_core::entity::squad::{Formation, Goal};
    pub use cordon_core::item::{ItemInstance, Loadout};
    pub use cordon_core::primitive::{Credits, Experience, Loyalty, Trust};

    pub use super::{CordonSimPlugin, SimSet};
    pub use crate::behavior::{
        AnomalyZone, CombatTarget, Dead, FireState, LootState, MovementSpeed, MovementTarget,
        Vision,
    };
    // Events are re-exported from their producer modules so
    // external consumers (cordon-bevy visuals, audio) can import
    // everything from the prelude without knowing the internal
    // module layout.
    pub use crate::combat::ShotFired;
    pub use crate::components::{
        BaseMaxes, Employment, FactionId, Hp, HungerPool, NpcAttributes, NpcBundle, NpcMarker,
        Perks, CorruptionPool, RelicHome, RelicMarker, SquadActivity, SquadBundle, SquadFacing,
        SquadHomePosition, SquadLeader, SquadMarker, SquadMembers, SquadMembership, SquadWaypoints,
        StaminaPool,
    };
    pub use crate::day::DayRolled;
    pub use crate::death::{CorpseRemoved, NpcDied};
    pub use crate::loot::ItemLooted;
    pub use crate::quest::{ActiveQuest, CompletedQuest, QuestLog, StartQuestRequest};
    pub use crate::resources::{
        AreaStates, EventLog, FactionIndex, GameClock, Player, SquadIdIndex, UidAllocator,
    };
    pub use crate::spawn::SquadSpawned;
    pub use crate::spawn::relics::RelicPickedUp;
    pub use crate::squad::{Owned, SquadCommand};
}
