//! Quest definitions and static metadata.
//!
//! Quests are linear narrative threads with occasional choice points,
//! driven through NPC dialogue, world conditions, and consequences.
//! All text is localised via stage IDs.
//!
//! Static definitions (`QuestDef`) live here. Runtime state
//! (`ActiveQuest`, `QuestLog`) lives in `cordon-sim` where it can
//! touch runtime resources like the dialogue runner's variable
//! storage and the game clock.
//!
//! # Authoring model
//!
//! A quest is a [`QuestDef`] plus one or more [`QuestTriggerDef`]s
//! that decide when it starts. Triggers live in their own table
//! (`assets/data/triggers/`) so a single quest can be reached
//! through multiple entry points (e.g. "on day 3" AND "when event
//! X fires") without duplicating stage definitions.
//!
//! Each stage is one of:
//!
//! - [`QuestStageKind::Talk`] — run a Yarn node; branch on the
//!   `$quest_choice` variable the node sets.
//! - [`QuestStageKind::Objective`] — wait for a world condition.
//! - [`QuestStageKind::Outcome`] — apply final consequences and end.
//!
//! Dialogue is Yarn-authoritative: option gating, text, and branch
//! selection all live in the `.yarn` file. The engine only reads
//! the final choice back via `$quest_choice` and maps it to the
//! next stage through [`TalkBranch`]s.

use serde::{Deserialize, Serialize};

use super::consequence::{Consequence, ObjectiveCondition};
use crate::entity::faction::Faction;
use crate::entity::npc::NpcTemplate;
use crate::primitive::{Day, Id, IdMarker};
use crate::world::event::Event;

/// Marker for quest definition IDs.
pub struct Quest;
impl IdMarker for Quest {}

/// Marker for quest stage IDs (unique within a quest).
pub struct QuestStage;
impl IdMarker for QuestStage {}

/// Marker for quest trigger definition IDs.
pub struct QuestTrigger;
impl IdMarker for QuestTrigger {}

/// Broad category for quest sorting and UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestCategory {
    /// Critical-path story quest.
    Main,
    /// Optional side story.
    Side,
    /// Faction-specific quest chain.
    Faction,
    /// Onboarding / tutorial quest.
    Tutorial,
}

/// One branch out of a [`QuestStageKind::Talk`] stage, keyed by
/// the value the Yarn node wrote to `$quest_choice`.
///
/// An empty [`choice`](TalkBranch::choice) is the linear-dialog
/// case (no options, just "run the node then move on").
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TalkBranch {
    /// Value of `$quest_choice` this branch matches.
    pub choice: String,
    /// Stage to advance to when this branch is taken.
    pub next_stage: Id<QuestStage>,
}

/// What happens at a quest stage.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestStageKind {
    /// Run a Yarn dialogue node. The engine enqueues a visitor
    /// (if `npc` is set), hands control to the dialogue runner,
    /// and on completion reads the `$quest_choice` Yarn variable
    /// and dispatches to the matching [`TalkBranch`]. If no branch
    /// matches, jumps to [`fallback`](QuestStageKind::Talk::fallback).
    Talk {
        /// Which NPC template delivers this line. `None` means
        /// narrator (no visitor enqueued).
        npc: Option<Id<NpcTemplate>>,
        /// Yarn node name to run.
        yarn_node: String,
        /// Available branches, matched against `$quest_choice`.
        branches: Vec<TalkBranch>,
        /// Stage to advance to if no branch matches (dialogue
        /// ended without writing a recognised `$quest_choice`).
        fallback: Id<QuestStage>,
    },

    /// Wait for a world condition to become true.
    Objective {
        /// What must be true to succeed. Supports `AllOf` / `AnyOf`
        /// / `Not` for compound conditions.
        condition: ObjectiveCondition,
        /// Minutes of game time before the objective expires.
        /// `None` means untimed.
        timeout_minutes: Option<u32>,
        /// Stage to advance to on success.
        on_success: Id<QuestStage>,
        /// Stage to advance to on failure / timeout. `None` means
        /// the quest ends in failure immediately.
        on_failure: Option<Id<QuestStage>>,
    },

    /// Terminal stage. Applies final consequences and records the
    /// completion in [`QuestLog`](super).
    Outcome {
        /// Whether this is a success ending.
        success: bool,
        /// Rewards, standing changes, follow-up quest starts, etc.
        consequences: Vec<Consequence>,
    },
}

/// A single stage in a quest.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestStageDef {
    /// Unique stage ID within this quest. Also the localisation
    /// key for any narrator text tied to this stage.
    pub id: Id<QuestStage>,
    /// What happens at this stage.
    pub kind: QuestStageKind,
}

/// A quest definition loaded from config.
///
/// Quests are sequences of stages referenced by stage ID. The
/// first element of [`stages`](QuestDef::stages) is the entry
/// point. The [`id`](QuestDef::id) doubles as the localisation
/// key for the quest name and description.
///
/// Quests do **not** store their own trigger conditions — those
/// live in [`QuestTriggerDef`]s in a separate table, so one quest
/// can have multiple entry points without duplicating its stages.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestDef {
    /// Unique identifier and localisation key.
    pub id: Id<Quest>,
    /// UI grouping: main / side / faction / tutorial.
    pub category: QuestCategory,
    /// Which NPC template gives out this quest (for intel UI and
    /// visitor spawning). `None` means the quest has no single giver
    /// (e.g. a world-triggered quest).
    pub giver: Option<Id<NpcTemplate>>,
    /// Faction credit for this quest (for intel UI grouping). May
    /// differ from [`giver`](QuestDef::giver)'s faction in
    /// cross-faction plots.
    pub giver_faction: Option<Id<Faction>>,
    /// Quest-wide time limit in game-minutes from start. `None`
    /// means untimed at the quest level (individual stages can
    /// still have per-stage timeouts).
    pub time_limit_minutes: Option<u32>,
    /// All stages in this quest. First stage is the entry point.
    /// Stages reference each other by stage ID, not by index.
    pub stages: Vec<QuestStageDef>,
    /// Whether this quest can be active multiple times
    /// simultaneously. Defaults to false — most quests are
    /// one-shot per campaign.
    #[serde(default)]
    pub repeatable: bool,
}

impl QuestDef {
    /// Look up a stage by its ID. `None` if the ID does not
    /// match any stage in this quest — typically a dangling
    /// reference flagged at load time by the sim layer's
    /// catalog validator.
    pub fn stage(&self, id: &Id<QuestStage>) -> Option<&QuestStageDef> {
        self.stages.iter().find(|s| &s.id == id)
    }

    /// Entry stage (the first element of [`stages`](Self::stages)).
    /// `None` when the quest has no stages, which is itself an
    /// authoring error and will also surface in validation.
    pub fn entry_stage(&self) -> Option<&QuestStageDef> {
        self.stages.first()
    }
}

/// How a quest gets triggered.
///
/// Stored in a table parallel to the quest definitions. A single
/// quest may have multiple triggers (e.g. "start on day 3" AND
/// "start when the player reaches Friendly with the Garrison").
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestTriggerKind {
    /// Fire on the very first tick of a new game.
    OnGameStart,
    /// Fire on the named day.
    OnDay(Day),
    /// Fire when the given event fires.
    OnEvent(Id<Event>),
    /// Fire when the given condition first becomes true.
    OnCondition(ObjectiveCondition),
}

/// A trigger rule that can start a quest.
///
/// Triggers live in their own table (`assets/data/triggers/`) so
/// a single [`QuestDef`] can be reached through multiple entry
/// points without duplicating its stages. A trigger fires only
/// once per game unless [`repeatable`](QuestTriggerDef::repeatable)
/// is set — the quest engine tracks fired-trigger IDs in the
/// quest log and skips anything it's already seen.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuestTriggerDef {
    /// Unique identifier for this trigger rule.
    pub id: Id<QuestTrigger>,
    /// Which quest this trigger starts.
    pub quest: Id<Quest>,
    /// What kind of event fires the trigger.
    pub kind: QuestTriggerKind,
    /// Extra prerequisite evaluated at trigger time. `None`
    /// means no gate beyond [`kind`](Self::kind). For
    /// conjunctions use [`ObjectiveCondition::AllOf`]; for
    /// disjunctions use [`AnyOf`](ObjectiveCondition::AnyOf).
    /// Collapsing this to a single [`ObjectiveCondition`]
    /// keeps one way to express boolean logic over world
    /// state — the same recursive vocabulary quest stages
    /// already use.
    #[serde(default)]
    pub requires: Option<ObjectiveCondition>,
    /// Whether this trigger can fire more than once per game.
    /// Defaults to false.
    #[serde(default)]
    pub repeatable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip the exact JSON shape used by
    /// `assets/data/quests/first_contact.json` through the real
    /// deserializer. Catches serde-tag mismatches (e.g. if
    /// `rename_all` drifts away from `snake_case`) without
    /// needing to boot the whole game.
    #[test]
    fn first_contact_json_parses() {
        let json = r#"{
            "id": "first_contact",
            "category": "tutorial",
            "giver": "garrison_lieutenant",
            "giver_faction": "garrison",
            "time_limit_minutes": null,
            "stages": [
                {
                    "id": "intro",
                    "kind": {
                        "talk": {
                            "npc": "garrison_lieutenant",
                            "yarn_node": "first_contact.intro",
                            "branches": [
                                { "choice": "accept",  "next_stage": "outcome_accept" },
                                { "choice": "neutral", "next_stage": "outcome_neutral" },
                                { "choice": "refuse",  "next_stage": "outcome_refuse" }
                            ],
                            "fallback": "outcome_refuse"
                        }
                    }
                },
                {
                    "id": "outcome_accept",
                    "kind": {
                        "outcome": {
                            "success": true,
                            "consequences": [
                                { "give_credits": 200 },
                                { "give_player_xp": 50 },
                                { "standing_change": { "faction": "garrison",  "delta": 15 } },
                                { "standing_change": { "faction": "syndicate", "delta": -5 } }
                            ]
                        }
                    }
                }
            ],
            "repeatable": false
        }"#;
        let def: QuestDef = serde_json::from_str(json).expect("parse first_contact");
        assert_eq!(def.id.as_str(), "first_contact");
        assert_eq!(def.stages.len(), 2);
        assert!(matches!(def.category, QuestCategory::Tutorial));
        let QuestStageKind::Talk { branches, .. } = &def.stages[0].kind else {
            panic!("stage 0 should be Talk");
        };
        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].choice, "accept");
    }

    #[test]
    fn trigger_json_parses_minimal() {
        let json = r#"{
            "id": "first_contact_intro",
            "quest": "first_contact",
            "kind": "on_game_start",
            "repeatable": false
        }"#;
        let trigger: QuestTriggerDef = serde_json::from_str(json).expect("parse trigger");
        assert_eq!(trigger.quest.as_str(), "first_contact");
        assert!(matches!(trigger.kind, QuestTriggerKind::OnGameStart));
        assert!(trigger.requires.is_none());
    }

    #[test]
    fn trigger_json_parses_with_requires_allof() {
        let json = r#"{
            "id": "lieutenant_visit",
            "quest": "first_contact",
            "kind": { "on_event": "faction_war" },
            "requires": {
                "all_of": [
                    { "faction_standing": { "faction": "garrison", "min_standing": 50 } },
                    { "quest_completed": "tutorial" }
                ]
            },
            "repeatable": false
        }"#;
        let trigger: QuestTriggerDef = serde_json::from_str(json).expect("parse trigger");
        assert!(matches!(trigger.kind, QuestTriggerKind::OnEvent(_)));
        assert!(matches!(
            trigger.requires,
            Some(ObjectiveCondition::AllOf(_))
        ));
    }
}
