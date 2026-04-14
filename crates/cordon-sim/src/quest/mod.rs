//! Runtime quest system.
//!
//! Static definitions (`QuestDef`, `QuestStageDef`, `QuestTriggerDef`)
//! live in `cordon-core`. This module owns the *runtime* side:
//!
//! - [`state`] — [`ActiveQuest`], [`CompletedQuest`], [`QuestLog`]
//! - [`condition`] — recursive [`ObjectiveCondition`] evaluator
//! - [`consequence`] — [`Consequence`] applier and
//!   [`StartQuestRequest`] message
//! - [`engine`] — trigger dispatch, objective driving, outcome
//!   application, stage-reference validation
//!
//! Quest talking happens via Yarn. The `Talk` stage driver lives
//! in cordon-bevy (next to the dialogue runner) because it
//! needs access to Yarn's variable storage and the visitor
//! queue. The cordon-sim side exposes
//! [`engine::advance_after_talk`] so the bridge can jump to the
//! next stage when Yarn hands control back.

pub mod condition;
pub mod consequence;
pub mod engine;
pub mod registry;
pub mod state;
pub mod travel;

use bevy::prelude::*;

pub use self::consequence::{GiveNpcXpRequest, SpawnNpcRequest, StartQuestRequest};
pub use self::registry::TemplateRegistry;
pub use self::state::{ActiveQuest, CompletedQuest, QuestLog};
pub use self::travel::{BunkerArrival, HomeArrival};
use crate::day::DayRolled;
use crate::plugin::SimSet;
use crate::resources::GameClock;

/// Bevy plugin: sets up the [`QuestLog`] resource, the
/// [`StartQuestRequest`] message, trigger dispatchers, and the
/// frame-driven quest engine.
///
/// All systems run inside [`SimSet::Cleanup`] — quests are
/// bookkeeping, not per-tick sim work, and clumping them with
/// the other daily housekeeping keeps the schedule tidy.
pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLog>();
        app.init_resource::<TemplateRegistry>();
        app.add_message::<StartQuestRequest>();
        app.add_message::<SpawnNpcRequest>();
        app.add_message::<GiveNpcXpRequest>();
        app.add_message::<BunkerArrival>();
        app.add_message::<HomeArrival>();
        app.add_message::<consequence::DismissTemplateNpc>();

        // Catalog validation lives in cordon-data now, running
        // inline inside `assemble_game_data` before the
        // `GameDataResource` is inserted — so no sim-side
        // one-shot system is needed.
        //
        // Game-start trigger dispatch runs once the sim is
        // fully bootstrapped (`GameClock`, inserted by
        // `init_world_resources` on `OnEnter(AppState::Playing)`
        // after the catalog is already live).
        app.add_systems(
            Update,
            engine::dispatch_on_game_start.run_if(resource_added::<GameClock>),
        );

        app.add_systems(
            Update,
            (
                engine::dispatch_on_day.run_if(on_message::<DayRolled>),
                engine::dispatch_on_event,
                engine::dispatch_on_condition,
                engine::drive_active_quests,
                engine::process_start_quest_requests,
                engine::fail_talk_on_template_death,
            )
                .chain()
                .in_set(SimSet::Cleanup)
                .run_if(resource_exists::<GameClock>),
        );

        // Arrival detection is a lightweight transform scan;
        // runs every Update outside the cleanup bundle so the
        // Bevy-layer arrival handler can consume the message
        // the same frame.
        app.add_systems(
            Update,
            (
                travel::detect_bunker_arrival,
                travel::detect_home_arrival,
                travel::prune_despawned_templates,
            ),
        );
    }
}
