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
use crate::day::DayCyclePlugin;
use crate::effects::EffectsPlugin;
use crate::quest::QuestPlugin;
use crate::resources::{GameClock, SquadIdIndex, UidAllocator};
use crate::spawn;
use crate::spawn::relics::RelicSpawnPlugin;

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
    /// `NpcPoolChanged`.
    Combat,
    /// Effect dispatch and active-effect ticking. Runs after
    /// combat so it can react to `NpcPoolChanged` messages, and
    /// before death handling so an `OnLowHealth` heal can pull a
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
        // BehaviorPlugin composes Movement / Vision / Combat /
        // Death / Loot / Squad subplugins internally, so we register
        // one plugin for that bundle.
        app.add_plugins((
            DayCyclePlugin,
            BehaviorPlugin,
            EffectsPlugin,
            RelicSpawnPlugin,
            QuestPlugin,
        ));
    }
}

/// Re-exports for convenience. Downstream crates (cordon-bevy
/// visuals, audio) import from this prelude rather than the
/// internal subplugin paths so structural changes here don't
/// ripple outward.
pub mod prelude {
    pub use super::{CordonSimPlugin, SimSet};

    // Behavior subplugin exports: each subplugin's component + event
    // types, grouped per subplugin so consumers can reason about
    // which subsystem an import comes from.
    pub use crate::behavior::combat::{CombatTarget, FireState, NpcPoolChanged, ShotFired};
    pub use crate::behavior::death::{CorpseRemoved, Dead, NpcDied};
    pub use crate::behavior::loot::{ItemLooted, LootState};
    pub use crate::behavior::movement::{MovementSpeed, MovementTarget};
    pub use crate::behavior::vision::{AnomalyZone, Vision};

    // Per-entity components not owned by a subplugin.
    pub use crate::entity::npc::{
        ActiveEffects, BaseMaxes, CorruptionPool, Employment, FactionId, HealthPool,
        NpcAttributes, NpcBundle, NpcMarker, PendingYarnNode, Perks, QuestCritical, SpawnOrigin,
        StaminaPool, TemplateId, TravelingHome, TravelingToBunker,
    };
    pub use crate::entity::relic::{RelicHome, RelicMarker};

    // Squad components and commands.
    pub use crate::behavior::squad::components::{
        EngagementTarget, MovementIntent, SquadBundle, SquadFacing, SquadHomePosition,
        SquadLeader, SquadMarker, SquadMembers, SquadMembership, SquadWaypoints,
    };
    pub use crate::behavior::squad::{Owned, SquadCommand};

    // Cross-cutting messages and resources.
    pub use crate::day::DayRolled;
    pub use crate::quest::{
        ActiveQuest, CompletedQuest, GiveNpcXpRequest, QuestLog, SpawnNpcRequest,
        StartQuestRequest, TemplateRegistry,
    };
    pub use crate::resources::{
        AreaStates, EventLog, FactionIndex, GameClock, Player, SquadIdIndex, UidAllocator,
    };
    pub use crate::spawn::SquadSpawned;
    pub use crate::spawn::relics::RelicPickedUp;

    // Cordon-core types that derive `Component` directly and are
    // attached to entities as live components, plus the flavour
    // types (`Trust`, `Loyalty`, `Personality`) that are bundled
    // inside `NpcAttributes`.
    pub use cordon_core::entity::name::NpcName;
    pub use cordon_core::entity::npc::Personality;
    pub use cordon_core::entity::squad::{Formation, Goal};
    pub use cordon_core::item::{ItemInstance, Loadout};
    pub use cordon_core::primitive::{Credits, Experience, Loyalty, Trust};
}
