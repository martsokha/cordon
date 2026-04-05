//! Quest definitions and runtime state.
//!
//! Quests are linear narrative threads with occasional choice points.
//! They play out through NPC dialogs, player choices, timed objectives,
//! and consequences. All text is localized via the stage/choice IDs.
//!
//! Quests can be triggered by events, faction standing, or other quests.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Faction;
use crate::entity::npc::NpcTemplate;
use crate::primitive::id::{Id, IdMarker};
use crate::primitive::time::Day;
use crate::world::narrative::consequence::{Consequence, ObjectiveCondition};

/// Marker for quest definition IDs.
pub struct Quest;
impl IdMarker for Quest {}

/// Marker for quest stage IDs (unique within a quest).
pub struct QuestStage;
impl IdMarker for QuestStage {}

/// Marker for quest choice option IDs.
pub struct QuestChoice;
impl IdMarker for QuestChoice {}

/// A single option in a choice stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceOption {
    /// Unique choice ID and localization key for the choice text.
    pub id: Id<QuestChoice>,
    /// Condition that must be met for this choice to appear.
    /// `None` means always available.
    pub requires: Option<ObjectiveCondition>,
    /// What happens when this choice is picked.
    pub consequences: Vec<Consequence>,
    /// Which stage to advance to after this choice.
    pub next_stage: Id<QuestStage>,
}

/// What happens at a quest stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestStageKind {
    /// NPC speaks to the player. Advances automatically.
    /// The stage [`id`](QuestStageDef::id) is the localization key
    /// for the dialog text.
    Dialog {
        /// Which NPC is speaking. `None` means narrator.
        npc: Option<Id<NpcTemplate>>,
        /// Stage to advance to after the dialog.
        next_stage: Id<QuestStage>,
    },

    /// Player picks from options. Each option has consequences
    /// and leads to a different next stage. Options can be gated
    /// by conditions.
    Choice {
        /// Which NPC is presenting the choice. `None` means narrator.
        npc: Option<Id<NpcTemplate>>,
        /// Available options (filtered at runtime by their `requires` field).
        options: Vec<ChoiceOption>,
        /// Days before the choice times out. `None` means wait forever.
        timeout_days: Option<u8>,
        /// Stage to advance to if the choice times out.
        /// Required if `timeout_days` is set.
        on_timeout: Option<Id<QuestStage>>,
    },

    /// Wait for a condition to be met. Can have a timeout.
    Objective {
        /// What needs to happen.
        condition: ObjectiveCondition,
        /// Days before the objective expires. `None` means no timeout.
        timeout_days: Option<u8>,
        /// Stage to advance to on success.
        on_success: Id<QuestStage>,
        /// Stage to advance to on failure/timeout. `None` means quest fails.
        on_failure: Option<Id<QuestStage>>,
    },

    /// Quest ends. Apply final consequences and record completion.
    Outcome {
        /// Whether this is a success or failure ending.
        success: bool,
        /// Final consequences (rewards, standing changes, etc.).
        consequences: Vec<Consequence>,
    },
}

/// A single stage in a quest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStageDef {
    /// Unique stage ID within this quest. Also the localization key
    /// for any text associated with this stage.
    pub id: Id<QuestStage>,
    /// What happens at this stage.
    pub kind: QuestStageKind,
}

/// A quest definition loaded from config.
///
/// Quests are linear sequences of stages with optional choice branches.
/// The first stage in [`stages`](QuestDef::stages) is the entry point.
/// The [`id`](QuestDef::id) doubles as the localization key for the
/// quest name/description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDef {
    /// Unique identifier and localization key.
    pub id: Id<Quest>,
    /// All stages in this quest. First stage is the entry point.
    /// Stages reference each other by stage ID, not by index.
    pub stages: Vec<QuestStageDef>,
    /// Whether this quest can be active multiple times simultaneously.
    pub repeatable: bool,
    /// Quest IDs that must be completed (successfully) before this
    /// quest can be triggered.
    pub requires_quests: Vec<Id<Quest>>,
    /// Minimum faction standings required to trigger this quest.
    /// All conditions must be met. Empty means no standing requirement.
    pub requires_standings: Vec<(Id<Faction>, i8)>,
}

/// A quest instance currently in progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveQuest {
    /// ID of the [`QuestDef`] this is an instance of.
    pub def_id: Id<Quest>,
    /// ID of the current stage within the quest.
    pub current_stage: Id<QuestStage>,
    /// Day this quest was started.
    pub day_started: Day,
    /// Day the current stage started (for timeout tracking).
    pub stage_started: Day,
    /// IDs of choices made so far (in order).
    pub choices_made: Vec<Id<QuestChoice>>,
}

/// A record of a completed quest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedQuest {
    /// ID of the [`QuestDef`] that was completed.
    pub def_id: Id<Quest>,
    /// Day the quest was started.
    pub day_started: Day,
    /// Day the quest was completed.
    pub day_completed: Day,
    /// Whether the quest ended successfully.
    pub success: bool,
    /// ID of the outcome stage that was reached.
    pub outcome_stage: Id<QuestStage>,
    /// All choices made during this quest (in order).
    pub choices_made: Vec<Id<QuestChoice>>,
}
