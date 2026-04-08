//! Runtime quest state.
//!
//! [`ActiveQuest`] tracks one in-progress quest instance: which
//! stage it's in, when it started, and a scratch bag of flags
//! that the Yarn dialogue bridge writes into. [`QuestLog`] is the
//! Bevy resource that owns all active and completed quest
//! instances plus the set of triggers that have already fired.
//!
//! Definitions (`QuestDef`, `QuestStageDef`, `QuestTriggerDef`)
//! live in `cordon-core` because they are pure data. Runtime
//! state lives here because it touches [`GameTime`] and
//! [`YarnValue`] — Bevy-only runtime types that should not bleed
//! into the static catalog.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_yarnspinner::prelude::YarnValue;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::quest::{Quest, QuestStage, QuestTrigger};

/// One in-progress quest instance.
///
/// Flags are a mirror of the subset of Yarn variables the
/// dialogue bridge copies across when a `Talk` stage completes.
/// Any variable prefixed with `$quest_` is captured; the
/// reserved `$quest_choice` is additionally used to pick the
/// next [`QuestStage`] out of the stage's `branches`.
#[derive(Debug, Clone)]
pub struct ActiveQuest {
    /// Which [`QuestDef`] this is an instance of.
    pub def_id: Id<Quest>,
    /// Current stage within the quest.
    pub current_stage: Id<QuestStage>,
    /// Game time at which the quest started. Used against
    /// [`QuestDef::time_limit_minutes`] for quest-wide timeouts.
    pub started_at: GameTime,
    /// Game time at which the current stage was entered. Used
    /// against each stage's own `timeout_minutes` where set.
    pub stage_started_at: GameTime,
    /// Yarn variables this quest has accumulated. Written by the
    /// dialogue bridge on stage completion; read by conditions
    /// that use [`ObjectiveCondition::QuestFlag`] and by Yarn
    /// itself on subsequent `Talk` stages.
    pub flags: HashMap<String, YarnValue>,
    /// Whether the quest is currently waiting for a dialogue
    /// round-trip. Set when a `Talk` stage enqueues a visitor;
    /// cleared when the Yarn bridge processes the
    /// `DialogueCompleted` event. Keeps the stage driver
    /// idempotent.
    pub awaiting_dialogue: bool,
}

impl ActiveQuest {
    /// Create a new instance at the given entry stage, with
    /// `started_at` and `stage_started_at` aligned to `now`.
    pub fn new(def_id: Id<Quest>, entry: Id<QuestStage>, now: GameTime) -> Self {
        Self {
            def_id,
            current_stage: entry,
            started_at: now,
            stage_started_at: now,
            flags: HashMap::new(),
            awaiting_dialogue: false,
        }
    }

    /// Advance to the given stage, resetting the stage timer.
    pub fn advance_to(&mut self, stage: Id<QuestStage>, now: GameTime) {
        self.current_stage = stage;
        self.stage_started_at = now;
        self.awaiting_dialogue = false;
    }
}

/// A record of a finished quest. Kept in [`QuestLog::completed`]
/// so later conditions can ask "was quest X completed?" and the
/// intel UI can render history.
#[derive(Debug, Clone)]
pub struct CompletedQuest {
    pub def_id: Id<Quest>,
    pub started_at: GameTime,
    pub completed_at: GameTime,
    /// Whether the quest ended in the success branch of its
    /// final [`QuestStageKind::Outcome`].
    pub success: bool,
    /// ID of the outcome stage that was reached.
    pub outcome_stage: Id<QuestStage>,
    /// Final flag state at completion. Preserved so quests that
    /// chain via `StartQuest` consequences can still read the
    /// predecessor's choices after it has ended.
    pub flags: HashMap<String, YarnValue>,
}

/// The world-owned quest book.
///
/// Holds every active quest instance, every completed quest
/// record, and the set of trigger IDs that have already fired —
/// non-repeatable triggers consult this set before re-firing.
#[derive(Resource, Debug, Default, Clone)]
pub struct QuestLog {
    /// Quests currently in progress.
    pub active: Vec<ActiveQuest>,
    /// Quests that have ended, successfully or otherwise.
    pub completed: Vec<CompletedQuest>,
    /// Triggers that have already fired at least once. A
    /// repeatable trigger may be in here *and* still eligible to
    /// fire again — the dispatcher only uses this set to gate
    /// non-repeatable triggers.
    pub fired_triggers: HashSet<Id<QuestTrigger>>,
}

impl QuestLog {
    /// Whether any active quest is currently an instance of the
    /// given definition. Used by the trigger dispatcher to avoid
    /// double-starting non-repeatable quests.
    pub fn is_active(&self, quest: &Id<Quest>) -> bool {
        self.active.iter().any(|q| &q.def_id == quest)
    }

    /// Whether the given quest has at least one successful
    /// completion on record.
    pub fn is_completed_successfully(&self, quest: &Id<Quest>) -> bool {
        self.completed
            .iter()
            .any(|c| &c.def_id == quest && c.success)
    }

    /// Find the active instance of the given quest definition,
    /// if any. Quests are not expected to run in parallel (the
    /// `repeatable` flag on [`QuestDef`](cordon_core::world::narrative::quest::QuestDef)
    /// allows it but the lookup simply returns the first match).
    pub fn active_instance(&self, quest: &Id<Quest>) -> Option<&ActiveQuest> {
        self.active.iter().find(|q| &q.def_id == quest)
    }

    /// Mutable variant of [`active_instance`](Self::active_instance).
    pub fn active_instance_mut(&mut self, quest: &Id<Quest>) -> Option<&mut ActiveQuest> {
        self.active.iter_mut().find(|q| &q.def_id == quest)
    }
}
