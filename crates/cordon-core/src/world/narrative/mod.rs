//! Narrative system: quests, consequences, conditions, events, triggers, intel.
//!
//! Five modules, one flat namespace. Callers import everything
//! through `cordon_core::world::narrative` — sub-module paths are
//! an implementation detail.
//!
//! - [`quest`] — quest definitions, stages, categories.
//! - [`consequence`] — `ObjectiveCondition` and `Consequence`,
//!   the shared vocabulary quest stages and events both use.
//! - [`event`] — zone event definitions, live instances, and radio entries.
//! - [`intel`] — data-driven intel definitions and categories.
//! - [`trigger`] — rules that start quests in response to world
//!   events, day rollovers, or condition state changes.

mod consequence;
mod decision;
mod event;
mod flag;
mod intel;
mod quest;
mod trigger;

pub use self::consequence::{ConditionalConsequence, Consequence, EndingCause, ObjectiveCondition};
pub use self::decision::{Decision, DecisionDef};
pub use self::event::{ActiveEvent, Event, EventDef, RadioEntry};
pub use self::flag::{QuestFlagPredicate, QuestFlagValue};
pub use self::intel::{Intel, IntelDef};
pub use self::quest::{
    BranchArm, BranchStage, ObjectiveStage, OutcomeStage, Quest, QuestCategory, QuestDef,
    QuestStage, QuestStageDef, QuestStageKind, TalkBranch, TalkStage,
};
pub use self::trigger::{QuestTrigger, QuestTriggerDef, QuestTriggerKind};
