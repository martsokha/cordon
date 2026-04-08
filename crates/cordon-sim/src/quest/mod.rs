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
pub mod state;

use bevy::prelude::*;

pub use self::consequence::StartQuestRequest;
pub use self::state::{ActiveQuest, CompletedQuest, QuestLog};
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
        app.add_message::<StartQuestRequest>();

        // One-shot systems: Bevy's `resource_added` run
        // condition fires exactly once, on the frame the
        // targeted resource first appears, so no `Local<bool>`
        // latches are needed.
        //
        // - Validation runs as soon as the catalog is loaded
        //   (`GameDataResource`), the earliest moment stage
        //   references can be checked.
        // - Game-start trigger dispatch runs once the sim is
        //   fully bootstrapped (`GameClock`, inserted by
        //   `init_world_resources` on `OnEnter(AppState::Playing)`
        //   after the catalog is already live).
        app.add_systems(
            Update,
            (
                engine::validate_trigger_references
                    .run_if(resource_added::<cordon_data::gamedata::GameDataResource>),
                engine::dispatch_on_game_start.run_if(resource_added::<GameClock>),
            ),
        );

        app.add_systems(
            Update,
            (
                engine::dispatch_on_day.run_if(on_message::<DayRolled>),
                engine::drive_active_quests,
                engine::process_start_quest_requests,
            )
                .chain()
                .in_set(SimSet::Cleanup)
                .run_if(resource_exists::<GameClock>),
        );
    }
}
