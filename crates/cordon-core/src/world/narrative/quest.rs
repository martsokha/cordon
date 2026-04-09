//! Quest definitions and static metadata.
//!
//! Quests are linear narrative threads with occasional choice points,
//! driven through NPC dialogue, world conditions, and consequences.
//! All text is localised via stage IDs.
//!
//! Static definitions ([`QuestDef`]) live here. Runtime state
//! (`ActiveQuest`, `QuestLog`) lives in `cordon-sim` where it can
//! touch runtime resources like the dialogue runner's variable
//! storage and the game clock.
//!
//! # Authoring model
//!
//! A quest is a [`QuestDef`] plus one or more
//! [`QuestTriggerDef`](super::QuestTriggerDef)s that decide when
//! it starts. Triggers live in their own table so a single
//! quest can be reached through multiple entry points without
//! duplicating stage definitions.
//!
//! Each stage is one of:
//!
//! - [`QuestStageKind::Talk`] — run a Yarn node; pick the next
//!   stage from a list of [`TalkBranch`]es based on Yarn's
//!   `$quest_choice` variable, optionally gated by a condition.
//! - [`QuestStageKind::Objective`] — wait for a world condition.
//! - [`QuestStageKind::Branch`] — pick a next stage by condition,
//!   without dialogue. Used for silent forks.
//! - [`QuestStageKind::Outcome`] — apply final consequences and end.
//!
//! Dialogue is Yarn-authoritative: option gating, text, and branch
//! selection all live in the `.yarn` file. The engine reads the
//! final choice back via `$quest_choice` and maps it to the next
//! stage through [`TalkBranch`]es, optionally skipping branches
//! whose [`requires`](TalkBranch::requires) guard is false.

use serde::{Deserialize, Serialize};

use super::consequence::{ConditionalConsequence, ObjectiveCondition};
use crate::entity::faction::Faction;
use crate::entity::npc::NpcTemplate;
use crate::primitive::{Duration, Id, IdMarker};

/// Marker for quest definition IDs.
pub struct Quest;
impl IdMarker for Quest {}

/// Marker for quest stage IDs (unique within a quest).
pub struct QuestStage;
impl IdMarker for QuestStage {}

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
///
/// [`requires`](TalkBranch::requires) gates the engine side of
/// branch selection: a branch whose guard is false is skipped
/// during matching, so authors can express "you only take this
/// branch if you also have the medkit" without having to teach
/// Yarn the rule. Yarn's own option gating is unaffected —
/// branches the author wants hidden from the player should be
/// hidden at the Yarn level too.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TalkBranch {
    /// Value of `$quest_choice` this branch matches.
    pub choice: String,
    /// Stage to advance to when this branch is taken.
    pub next_stage: Id<QuestStage>,
    /// Optional engine-side gate. `None` means always eligible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<ObjectiveCondition>,
}

/// One arm of a [`QuestStageKind::Branch`] stage.
///
/// The engine walks [`arms`](QuestStageKind::Branch::arms) in
/// order, takes the first arm whose [`when`](BranchArm::when)
/// evaluates true, and advances to its [`next_stage`](BranchArm::next_stage).
/// If no arm matches, control falls through to the Branch
/// stage's own [`fallback`](QuestStageKind::Branch::fallback).
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BranchArm {
    /// Condition to match against the world view.
    pub when: ObjectiveCondition,
    /// Stage to advance to when this arm is taken.
    pub next_stage: Id<QuestStage>,
}

/// What happens at a quest stage.
///
/// Internally tagged by the `kind` field so JSON stays flat:
/// `{ "kind": "talk", "yarn_node": "...", ... }`. [`QuestStageDef`]
/// uses `#[serde(flatten)]` so the tag lives directly on the
/// stage object and not nested under a `kind` key.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuestStageKind {
    /// Run a Yarn dialogue node. The engine enqueues a visitor
    /// (if `npc` is set), hands control to the dialogue runner,
    /// and on completion reads the `$quest_choice` Yarn variable
    /// and dispatches to the first eligible [`TalkBranch`]. If
    /// no branch matches, jumps to [`fallback`](QuestStageKind::Talk::fallback).
    Talk {
        /// Which NPC template delivers this line. `None` means
        /// narrator (no visitor enqueued).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        npc: Option<Id<NpcTemplate>>,
        /// Yarn node name to run.
        yarn_node: String,
        /// Available branches, matched against `$quest_choice`.
        branches: Vec<TalkBranch>,
        /// Stage to advance to if no branch matches (dialogue
        /// ended without writing a recognised `$quest_choice`,
        /// or all eligible branches were gated out by
        /// [`TalkBranch::requires`]).
        fallback: Id<QuestStage>,
    },

    /// Wait for a world condition to become true.
    Objective {
        /// What must be true to succeed. Supports `AllOf` / `AnyOf`
        /// / `Not` for compound conditions.
        condition: ObjectiveCondition,
        /// Maximum stage lifetime. `None` means untimed. When
        /// the elapsed time exceeds this, the engine jumps to
        /// [`on_failure`](QuestStageKind::Objective::on_failure).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout: Option<Duration>,
        /// Stage to advance to on success.
        on_success: Id<QuestStage>,
        /// Stage to advance to on failure / timeout. `None` means
        /// the quest ends in failure immediately.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_failure: Option<Id<QuestStage>>,
    },

    /// Silent condition fork. The engine walks
    /// [`arms`](QuestStageKind::Branch::arms) in order and picks
    /// the first one whose [`when`](BranchArm::when) is true,
    /// falling through to [`fallback`](QuestStageKind::Branch::fallback)
    /// when nothing matches.
    ///
    /// Useful for "after the talk, go to different stages based
    /// on player state" without a UI step. Unlike `Objective`,
    /// Branch does not wait — it evaluates immediately on entry
    /// and advances the same frame.
    Branch {
        /// Condition arms in priority order.
        arms: Vec<BranchArm>,
        /// Stage to advance to when no arm matches.
        fallback: Id<QuestStage>,
    },

    /// Terminal stage. Applies consequences from every
    /// [`ConditionalConsequence`] whose guard matches and
    /// records the completion in `QuestLog`.
    Outcome {
        /// Whether this is a success ending.
        success: bool,
        /// Consequence bundles keyed by optional guards. Each
        /// eligible bundle fires in order; bundles with `None`
        /// for their guard always fire.
        consequences: Vec<ConditionalConsequence>,
    },
}

/// A single stage in a quest.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
pub struct QuestStageDef {
    /// Unique stage ID within this quest. Also the localisation
    /// key for any narrator text tied to this stage.
    pub id: Id<QuestStage>,
    /// What happens at this stage. Flattened so the
    /// internally-tagged `QuestStageKind` hoists its `kind` tag
    /// up to the stage object — authoring sees one flat JSON
    /// shape per stage instead of a nested `"kind": { "talk": ... }`.
    #[serde(flatten)]
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
/// live in [`QuestTriggerDef`](super::QuestTriggerDef)s in a
/// separate table, so one quest can have multiple entry points
/// without duplicating its stages.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giver: Option<Id<NpcTemplate>>,
    /// Faction credit for this quest (for intel UI grouping). May
    /// differ from [`giver`](QuestDef::giver)'s faction in
    /// cross-faction plots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub giver_faction: Option<Id<Faction>>,
    /// Quest-wide time limit measured from the moment the quest
    /// starts. `None` means untimed at the quest level (individual
    /// stages can still have per-stage timeouts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_limit: Option<Duration>,
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
            "stages": [
                {
                    "id": "intro",
                    "kind": "talk",
                    "npc": "garrison_lieutenant",
                    "yarn_node": "first_contact.intro",
                    "branches": [
                        { "choice": "accept",  "next_stage": "outcome_accept" },
                        { "choice": "neutral", "next_stage": "outcome_neutral" },
                        { "choice": "refuse",  "next_stage": "outcome_refuse" }
                    ],
                    "fallback": "outcome_refuse"
                },
                {
                    "id": "outcome_accept",
                    "kind": "outcome",
                    "success": true,
                    "consequences": [
                        { "apply": [
                            { "give_credits": 200 },
                            { "give_player_xp": 50 },
                            { "standing_change": { "faction": "garrison",  "delta": 15 } },
                            { "standing_change": { "faction": "syndicate", "delta": -5 } }
                        ]}
                    ]
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
        assert!(branches[0].requires.is_none());
    }

    #[test]
    fn objective_timeout_parses_from_duration_dsl() {
        let json = r#"{
            "id": "relic_hunt",
            "category": "side",
            "time_limit": { "hours": 2 },
            "stages": [
                {
                    "id": "gather",
                    "kind": "objective",
                    "condition": {
                        "have_item": { "item": "relic_crystal", "count": 3 }
                    },
                    "timeout": { "hours": 1 },
                    "on_success": "outcome_win",
                    "on_failure": "outcome_lose"
                },
                {
                    "id": "outcome_win",
                    "kind": "outcome",
                    "success": true,
                    "consequences": []
                },
                {
                    "id": "outcome_lose",
                    "kind": "outcome",
                    "success": false,
                    "consequences": []
                }
            ]
        }"#;
        let def: QuestDef = serde_json::from_str(json).expect("parse relic_hunt");
        assert_eq!(def.time_limit.unwrap().minutes(), 120);
        let QuestStageKind::Objective { timeout, .. } = &def.stages[0].kind else {
            panic!("stage 0 should be Objective");
        };
        assert_eq!(timeout.unwrap().minutes(), 60);
    }

    #[test]
    fn branch_stage_parses() {
        let json = r#"{
            "id": "silent_fork",
            "category": "side",
            "stages": [
                {
                    "id": "decide",
                    "kind": "branch",
                    "arms": [
                        {
                            "when": { "have_credits": 1000 },
                            "next_stage": "rich_path"
                        }
                    ],
                    "fallback": "poor_path"
                },
                { "id": "rich_path", "kind": "outcome", "success": true, "consequences": [] },
                { "id": "poor_path", "kind": "outcome", "success": true, "consequences": [] }
            ]
        }"#;
        let def: QuestDef = serde_json::from_str(json).expect("parse branch quest");
        let QuestStageKind::Branch { arms, fallback } = &def.stages[0].kind else {
            panic!("stage 0 should be Branch");
        };
        assert_eq!(arms.len(), 1);
        assert_eq!(arms[0].next_stage.as_str(), "rich_path");
        assert_eq!(fallback.as_str(), "poor_path");
    }

    #[test]
    fn conditional_outcome_consequences_parse() {
        let json = r#"{
            "id": "guarded_outcome",
            "category": "side",
            "stages": [
                {
                    "id": "terminal",
                    "kind": "outcome",
                    "success": true,
                    "consequences": [
                        { "apply": [ { "give_credits": 100 } ] },
                        {
                            "when": {
                                "faction_standing": { "faction": "garrison", "min_standing": 50 }
                            },
                            "apply": [ { "give_credits": 200 } ]
                        }
                    ]
                }
            ]
        }"#;
        let def: QuestDef = serde_json::from_str(json).expect("parse guarded_outcome");
        let QuestStageKind::Outcome { consequences, .. } = &def.stages[0].kind else {
            panic!("stage 0 should be Outcome");
        };
        assert_eq!(consequences.len(), 2);
        assert!(consequences[0].when.is_none());
        assert!(consequences[1].when.is_some());
    }
}
