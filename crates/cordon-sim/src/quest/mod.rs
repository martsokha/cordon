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
        app.add_systems(
            Update,
            (
                engine::validate_trigger_references,
                engine::dispatch_on_game_start,
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
