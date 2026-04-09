//! Quest trigger rules.
//!
//! Triggers live in their own table so one [`QuestDef`](super::quest::QuestDef)
//! can be reached through multiple entry points without duplicating
//! its stage definitions. The [`QuestTriggerKind`] picks *when* to
//! fire; the optional [`requires`](QuestTriggerDef::requires) gate
//! adds world-state prerequisites on top.

use serde::{Deserialize, Serialize};

use super::consequence::ObjectiveCondition;
use super::event::Event;
use super::quest::Quest;
use crate::primitive::{Day, Id, IdMarker};

/// Marker for quest trigger definition IDs.
pub struct QuestTrigger;
impl IdMarker for QuestTrigger {}

/// How a quest gets triggered.
///
/// Stored in a table parallel to the quest definitions. A single
/// quest may have multiple triggers (e.g. "start on day 3" AND
/// "start when the player reaches Friendly with the Garrison").
///
/// # Re-evaluation semantics
///
/// Each variant has its own rule for what happens when the
/// trigger's moment arrives but its [`requires`](QuestTriggerDef::requires)
/// gate is not yet satisfied:
///
/// - [`OnGameStart`](Self::OnGameStart): discarded. There is
///   no second game start.
/// - [`OnDay`](Self::OnDay): discarded. The day passed.
/// - [`OnEvent`](Self::OnEvent): skipped on *this* firing, but
///   eligible again the next time the event transitions from
///   inactive to active. Effectively "fire when the event
///   next happens *and* the condition holds".
/// - [`OnCondition`](Self::OnCondition): keeps watching. The
///   rising edge is keyed on the composite `kind ∧ requires`,
///   so the trigger fires the next frame both become true
///   simultaneously — even if `kind` was already true first.
///
/// Authors who want a day / game-start trigger to wait for
/// additional prerequisites should express that as an
/// `OnCondition` with the full compound condition instead of
/// relying on `requires` re-evaluation.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestTriggerKind {
    /// Fire on the very first tick of a new game. Discards on
    /// `requires`-failure — see type-level docs.
    OnGameStart,
    /// Fire on the named day. Discards on `requires`-failure —
    /// see type-level docs.
    OnDay(Day),
    /// Fire each time the given event transitions from
    /// inactive to active. `requires`-failure is a skip, not a
    /// latch — the next firing gets another chance.
    OnEvent(Id<Event>),
    /// Fire on the rising edge of `kind ∧ requires` (see
    /// type-level docs). Plain `OnCondition(cond)` with no
    /// `requires` is just "fire when `cond` first becomes
    /// true".
    OnCondition(ObjectiveCondition),
}

/// A trigger rule that can start a quest.
///
/// Triggers live in their own table (`assets/data/triggers/`) so
/// a single [`QuestDef`](super::quest::QuestDef) can be reached
/// through multiple entry points without duplicating its stages.
/// A trigger fires only once per game unless
/// [`repeatable`](QuestTriggerDef::repeatable) is set — the quest
/// engine tracks fired-trigger IDs in the quest log and skips
/// anything it's already seen.
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
