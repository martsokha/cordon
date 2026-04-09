//! Narrative system: quests, consequences, conditions, events, triggers.
//!
//! Four modules, one flat namespace. Callers import everything
//! through `cordon_core::world::narrative` — sub-module paths are
//! an implementation detail.
//!
//! - [`quest`] — quest definitions, stages, categories.
//! - [`consequence`] — `ObjectiveCondition` and `Consequence`,
//!   the shared vocabulary quest stages and events both use.
//! - [`event`] — zone event definitions and live instances.
//! - [`trigger`] — rules that start quests in response to world
//!   events, day rollovers, or condition state changes.

mod consequence;
mod event;
mod flag;
mod quest;
mod trigger;

pub use self::consequence::{Consequence, ObjectiveCondition};
pub use self::event::{ActiveEvent, Event, EventCategory, EventDef};
pub use self::flag::{QuestFlagPredicate, QuestFlagValue};
pub use self::quest::{
    Quest, QuestCategory, QuestDef, QuestStage, QuestStageDef, QuestStageKind, TalkBranch,
};
pub use self::trigger::{QuestTrigger, QuestTriggerDef, QuestTriggerKind};
