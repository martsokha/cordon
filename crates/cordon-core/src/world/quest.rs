//! Quest definitions and runtime state.
//!
//! Quests are linear narrative threads with occasional choice points.
//! They play out through NPC dialogs, player choices, timed objectives,
//! and consequences. All text is localized via the stage/choice [`Id`]s.
//!
//! Quests can be triggered by events, faction standing, or other quests.

use serde::{Deserialize, Serialize};

use crate::primitive::id::Id;
use crate::world::time::Day;

/// A condition that must be met for an objective stage to complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveCondition {
    /// Player must have a specific item in storage.
    HaveItem(Id),
    /// Player must have at least this many credits.
    HaveCredits(u32),
    /// Player must reach a minimum standing with a faction.
    FactionStanding { faction: Id, min_standing: i8 },
    /// Player must have a specific upgrade installed.
    HaveUpgrade(Id),
    /// A specific event must be active in the world.
    EventActive(Id),
    /// Player must deliver a specific item to the quest NPC.
    DeliverItem(Id),
    /// Simply wait (used with timeout_days on the stage).
    Wait,
}

/// A consequence applied when a choice is made or a stage completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Consequence {
    /// Change standing with a faction.
    StandingChange { faction: Id, delta: i8 },
    /// Give credits to the player.
    GiveCredits(u32),
    /// Take credits from the player.
    TakeCredits(u32),
    /// Give an item to the player (placed in storage).
    GiveItem(Id),
    /// Remove an item from the player's storage.
    TakeItem(Id),
    /// Trigger an event by its def ID.
    TriggerEvent(Id),
    /// Start another quest.
    StartQuest(Id),
    /// Unlock an upgrade (make it available for purchase/installation).
    UnlockUpgrade(Id),
    /// Spawn a named NPC visitor (references an NPC template ID from config).
    SpawnNpc(Id),
    /// Immediately fail the current quest.
    FailQuest,
}

/// A single option in a choice stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceOption {
    /// Localization key for the choice text.
    pub id: Id,
    /// What happens when this choice is picked.
    pub consequences: Vec<Consequence>,
    /// Which stage to advance to after this choice.
    pub next_stage: Id,
}

/// What happens at a quest stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestStageKind {
    /// NPC speaks to the player. Advances automatically.
    /// The stage [`id`](QuestStageDef::id) is the localization key
    /// for the dialog text.
    Dialog {
        /// Which NPC is speaking. `None` means narrator.
        npc: Option<Id>,
        /// Stage to advance to after the dialog.
        next_stage: Id,
    },

    /// Player picks from options. Each option has consequences
    /// and leads to a different next stage.
    Choice {
        /// Which NPC is presenting the choice. `None` means narrator.
        npc: Option<Id>,
        /// Available options.
        options: Vec<ChoiceOption>,
        /// Days before the choice times out. `None` means wait forever.
        timeout_days: Option<u8>,
        /// Stage to advance to if the choice times out.
        /// Required if `timeout_days` is set.
        on_timeout: Option<Id>,
    },

    /// Wait for a condition to be met. Can have a timeout.
    Objective {
        /// What needs to happen.
        condition: ObjectiveCondition,
        /// Days before the objective expires. `None` means no timeout.
        timeout_days: Option<u8>,
        /// Stage to advance to on success.
        on_success: Id,
        /// Stage to advance to on failure/timeout. `None` means quest fails.
        on_failure: Option<Id>,
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
    /// Unique stage identifier within this quest. Also the
    /// localization key for any text associated with this stage.
    pub id: Id,
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
    pub id: Id,
    /// All stages in this quest. First stage is the entry point.
    /// Stages reference each other by ID, not by index.
    pub stages: Vec<QuestStageDef>,
    /// Whether this quest can be active multiple times simultaneously.
    pub repeatable: bool,
    /// Quest IDs that must be completed (successfully) before this
    /// quest can be triggered.
    pub requires_quests: Vec<Id>,
    /// Minimum faction standing required to trigger this quest.
    /// `None` means no standing requirement.
    pub requires_standing: Option<(Id, i8)>,
}

/// A quest instance currently in progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveQuest {
    /// ID of the [`QuestDef`] this is an instance of.
    pub def_id: Id,
    /// ID of the current stage within the quest.
    pub current_stage: Id,
    /// Day this quest was started.
    pub day_started: Day,
    /// Day the current stage started (for timeout tracking).
    pub stage_started: Day,
    /// IDs of choices made so far (in order). Each entry is the
    /// [`ChoiceOption::id`] that was selected.
    pub choices_made: Vec<Id>,
}

/// A record of a completed quest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedQuest {
    /// ID of the [`QuestDef`] that was completed.
    pub def_id: Id,
    /// Day the quest was started.
    pub day_started: Day,
    /// Day the quest was completed.
    pub day_completed: Day,
    /// Whether the quest ended successfully.
    pub success: bool,
    /// ID of the outcome stage that was reached.
    pub outcome_stage: Id,
    /// All choices made during this quest (in order).
    pub choices_made: Vec<Id>,
}
