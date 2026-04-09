//! Pure [`ObjectiveCondition`] evaluation.
//!
//! Single-entry recursive evaluator over world state. Quest
//! stages, quest triggers, and anywhere else that needs a
//! boolean over player + world state all route through
//! [`evaluate`]. Extending the vocabulary means adding one arm
//! here and nothing else.

use bevy::log::warn;
use bevy_yarnspinner::prelude::YarnValue;
use cordon_core::entity::player::PlayerState;
use cordon_core::primitive::GameTime;
use cordon_core::world::narrative::{
    ActiveEvent, ObjectiveCondition, QuestFlagPredicate, QuestFlagValue,
};

use super::state::QuestLog;

/// Live world state the evaluator reads from. Kept as a single
/// struct so [`evaluate`](Self::evaluate) has one clean receiver
/// instead of a growing tuple of references, and so adding a new
/// piece of evaluator context is a field-add instead of threading
/// an argument through every caller.
pub struct WorldView<'a> {
    pub player: &'a PlayerState,
    pub events: &'a [ActiveEvent],
    pub quests: &'a QuestLog,
    /// The current game clock. Used by `Wait` and any future
    /// time-sensitive leaf conditions.
    pub now: GameTime,
    /// When the current quest stage was entered, if the
    /// evaluator is running inside a stage context. `None`
    /// means the caller is a trigger `requires`, a standalone
    /// check, or somewhere else with no per-stage clock — any
    /// stage-aware condition falls back to a warning + false.
    pub stage_started_at: Option<GameTime>,
}

impl<'a> WorldView<'a> {
    /// Evaluate a condition against this view.
    ///
    /// Recursive for [`AllOf`](ObjectiveCondition::AllOf),
    /// [`AnyOf`](ObjectiveCondition::AnyOf), and
    /// [`Not`](ObjectiveCondition::Not). Leaf conditions do
    /// simple lookups against the player state, event log, or
    /// quest log.
    pub fn evaluate(&self, cond: &ObjectiveCondition) -> bool {
        match cond {
            ObjectiveCondition::HaveItem(q) => {
                self.player.has_item(&q.item, q.resolved_count(), q.scope)
            }
            ObjectiveCondition::HaveCredits(amount) => self.player.credits.can_afford(*amount),
            ObjectiveCondition::FactionStanding {
                faction,
                min_standing,
            } => self.player.standing(faction) >= *min_standing,
            ObjectiveCondition::HaveUpgrade(upgrade) => self.player.has_upgrade(upgrade),
            ObjectiveCondition::EventActive(event) => {
                self.events.iter().any(|e| &e.def_id == event)
            }
            ObjectiveCondition::QuestActive(quest) => self.quests.is_active(quest),
            ObjectiveCondition::QuestCompleted(quest) => {
                self.quests.is_completed_successfully(quest)
            }
            ObjectiveCondition::QuestFlag {
                quest,
                key,
                predicate,
            } => {
                // Active quest first, then most-recent completed
                // instance. Completed-quest fallback lets later
                // quests branch on how an earlier one ended.
                let value = self
                    .quests
                    .active_instance(quest)
                    .and_then(|a| a.flags.get(key))
                    .or_else(|| {
                        self.quests
                            .completed
                            .iter()
                            .rev()
                            .find(|c| &c.def_id == quest)
                            .and_then(|c| c.flags.get(key))
                    });
                match (predicate, value) {
                    // `IsSet` is true iff any value is present.
                    (QuestFlagPredicate::IsSet, v) => v.is_some(),
                    (_, None) => false,
                    (QuestFlagPredicate::Equals(expected), Some(v)) => {
                        yarn_value_equals(v, expected)
                    }
                    (QuestFlagPredicate::NotEquals(expected), Some(v)) => {
                        !yarn_value_equals(v, expected)
                    }
                    (QuestFlagPredicate::GreaterThan(threshold), Some(v)) => {
                        yarn_value_as_number(v).is_some_and(|n| n > *threshold)
                    }
                    (QuestFlagPredicate::LessThan(threshold), Some(v)) => {
                        yarn_value_as_number(v).is_some_and(|n| n < *threshold)
                    }
                }
            }

            // NPC-template conditions are stubs until the template
            // → live-entity resolution story lands (#104/#105).
            // The warning ships the ID so malformed quests surface
            // during development.
            ObjectiveCondition::NpcAlive(npc) => {
                warn!(
                    "STUB CONDITION: NpcAlive({}) evaluated — returning false",
                    npc.as_str()
                );
                false
            }
            ObjectiveCondition::NpcDead(npc) => {
                warn!(
                    "STUB CONDITION: NpcDead({}) evaluated — returning false",
                    npc.as_str()
                );
                false
            }
            ObjectiveCondition::NpcAtLocation { npc, area } => {
                warn!(
                    "STUB CONDITION: NpcAtLocation({}, {}) evaluated — returning false",
                    npc.as_str(),
                    area.as_str()
                );
                false
            }

            ObjectiveCondition::Wait { duration } => {
                let Some(started_at) = self.stage_started_at else {
                    // Evaluator was called without a stage clock
                    // — e.g. from a trigger `requires`. `Wait`
                    // is only meaningful inside a stage;
                    // anywhere else is an authoring mistake.
                    warn!("Wait condition evaluated without a stage clock — returning false");
                    return false;
                };
                let elapsed = self.now.minutes_since(started_at);
                elapsed >= duration.minutes()
            }

            ObjectiveCondition::AllOf(conds) => conds.iter().all(|c| self.evaluate(c)),
            ObjectiveCondition::AnyOf(conds) => conds.iter().any(|c| self.evaluate(c)),
            ObjectiveCondition::Not(inner) => !self.evaluate(inner),
        }
    }
}

/// Compare a live [`YarnValue`] flag against an authored
/// [`QuestFlagValue`] using Yarn's loose casting rules.
///
/// The three variants line up one-to-one: numbers to numbers
/// (parsing a number flag from its own representation rounds-
/// trips via `==`), booleans to booleans, strings to strings.
/// Cross-type comparisons return false rather than coerce —
/// authors can use the explicit predicate shape to say what
/// they mean.
fn yarn_value_equals(value: &YarnValue, expected: &QuestFlagValue) -> bool {
    match (value, expected) {
        (YarnValue::String(a), QuestFlagValue::String(b)) => a == b,
        (YarnValue::Number(a), QuestFlagValue::Number(b)) => a == b,
        (YarnValue::Boolean(a), QuestFlagValue::Boolean(b)) => a == b,
        _ => false,
    }
}

/// Coerce a live [`YarnValue`] flag to an `f32` for numeric
/// comparison predicates. `None` when the flag isn't numeric —
/// string-to-number coercion is deliberately not supported; use
/// an explicit numeric flag.
fn yarn_value_as_number(value: &YarnValue) -> Option<f32> {
    match value {
        YarnValue::Number(n) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy_yarnspinner::prelude::YarnValue;
    use cordon_core::entity::faction::Faction;
    use cordon_core::entity::player::PlayerState;
    use cordon_core::primitive::{Credits, Duration, GameTime, Id, Relation, RelationDelta};
    use cordon_core::world::narrative::{
        ObjectiveCondition, Quest, QuestFlagPredicate, QuestFlagValue, QuestStage,
    };

    use super::{WorldView, yarn_value_equals};
    use crate::quest::state::{ActiveQuest, CompletedQuest, QuestLog};

    fn player(factions: &[&str]) -> PlayerState {
        let ids: Vec<Id<Faction>> = factions.iter().map(|f| Id::<Faction>::new(*f)).collect();
        PlayerState::new(&ids)
    }

    fn view<'a>(player: &'a PlayerState, log: &'a QuestLog) -> WorldView<'a> {
        WorldView {
            player,
            events: &[],
            quests: log,
            now: GameTime::new(),
            stage_started_at: None,
        }
    }

    fn view_with_clock<'a>(
        player: &'a PlayerState,
        log: &'a QuestLog,
        now: GameTime,
        stage_started_at: GameTime,
    ) -> WorldView<'a> {
        WorldView {
            player,
            events: &[],
            quests: log,
            now,
            stage_started_at: Some(stage_started_at),
        }
    }

    #[test]
    fn have_credits_threshold() {
        let mut p = player(&[]);
        p.credits = Credits::new(500);
        let log = QuestLog::default();
        let v = view(&p, &log);
        assert!(v.evaluate(&ObjectiveCondition::HaveCredits(Credits::new(500))));
        assert!(v.evaluate(&ObjectiveCondition::HaveCredits(Credits::new(499))));
        assert!(!v.evaluate(&ObjectiveCondition::HaveCredits(Credits::new(501))));
    }

    #[test]
    fn faction_standing_at_threshold() {
        let mut p = player(&["garrison"]);
        if let Some(s) = p.standing_mut(&Id::<Faction>::new("garrison")) {
            s.apply(RelationDelta::new(50));
        }
        let log = QuestLog::default();
        let v = view(&p, &log);
        assert!(v.evaluate(&ObjectiveCondition::FactionStanding {
            faction: Id::<Faction>::new("garrison"),
            min_standing: Relation::new(50),
        }));
        assert!(!v.evaluate(&ObjectiveCondition::FactionStanding {
            faction: Id::<Faction>::new("garrison"),
            min_standing: Relation::new(60),
        }));
    }

    #[test]
    fn all_of_short_circuits_on_false() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::AllOf(vec![
            ObjectiveCondition::Wait {
                duration: Duration::INSTANT,
            },
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
        ]);
        // Wait{INSTANT} needs a stage clock, use the clocked view.
        let v = view_with_clock(&p, &log, GameTime::new(), GameTime::new());
        assert!(!v.evaluate(&cond));
    }

    #[test]
    fn any_of_short_circuits_on_true() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::AnyOf(vec![
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
            ObjectiveCondition::Wait {
                duration: Duration::INSTANT,
            },
        ]);
        let v = view_with_clock(&p, &log, GameTime::new(), GameTime::new());
        assert!(v.evaluate(&cond));
    }

    #[test]
    fn not_flips_result() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::Not(Box::new(ObjectiveCondition::Wait {
            duration: Duration::INSTANT,
        }));
        let v = view_with_clock(&p, &log, GameTime::new(), GameTime::new());
        assert!(!v.evaluate(&cond));
    }

    #[test]
    fn wait_without_stage_clock_returns_false() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::Wait {
            duration: Duration::INSTANT,
        };
        // No stage clock → false.
        assert!(!view(&p, &log).evaluate(&cond));
    }

    #[test]
    fn wait_honours_duration() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::Wait {
            duration: Duration::from_minutes(30),
        };
        let start = GameTime::new();
        // Zero elapsed → not yet satisfied.
        assert!(!view_with_clock(&p, &log, start, start).evaluate(&cond));
        // 30+ minutes elapsed → satisfied.
        let mut now_late = start;
        now_late.advance_minutes(30);
        assert!(view_with_clock(&p, &log, now_late, start).evaluate(&cond));
    }

    #[test]
    fn quest_active_lookup() {
        let p = player(&[]);
        let mut log = QuestLog::default();
        log.active.push(ActiveQuest {
            def_id: Id::<Quest>::new("mainline"),
            current_stage: Id::<QuestStage>::new("intro"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags: HashMap::new(),
        });
        let v = view(&p, &log);
        assert!(v.evaluate(&ObjectiveCondition::QuestActive(Id::<Quest>::new(
            "mainline"
        ))));
        assert!(!v.evaluate(&ObjectiveCondition::QuestActive(Id::<Quest>::new(
            "sidequest"
        ))));
    }

    #[test]
    fn quest_flag_equals_active_first_then_completed() {
        let p = player(&[]);
        let mut log = QuestLog::default();

        let mut flags = HashMap::new();
        flags.insert(
            "$quest_choice".to_string(),
            YarnValue::String("accepted".to_string()),
        );
        log.active.push(ActiveQuest {
            def_id: Id::<Quest>::new("live"),
            current_stage: Id::<QuestStage>::new("stage1"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags,
        });

        let mut completed_flags = HashMap::new();
        completed_flags.insert(
            "$quest_choice".to_string(),
            YarnValue::String("refused".to_string()),
        );
        log.completed.push(CompletedQuest {
            def_id: Id::<Quest>::new("done"),
            started_at: GameTime::new(),
            completed_at: GameTime::new(),
            success: false,
            outcome_stage: Id::<QuestStage>::new("outcome_refuse"),
            flags: completed_flags,
        });

        let v = view(&p, &log);
        // Active quest hit.
        assert!(v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$quest_choice".to_string(),
            predicate: QuestFlagPredicate::Equals(QuestFlagValue::String("accepted".to_string())),
        }));
        // Completed quest hit — active_instance miss falls through.
        assert!(v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("done"),
            key: "$quest_choice".to_string(),
            predicate: QuestFlagPredicate::Equals(QuestFlagValue::String("refused".to_string())),
        }));
    }

    #[test]
    fn quest_flag_is_set_matches_any_value() {
        let p = player(&[]);
        let mut log = QuestLog::default();
        let mut flags = HashMap::new();
        flags.insert("$quest_stage".to_string(), YarnValue::Number(3.0));
        log.active.push(ActiveQuest {
            def_id: Id::<Quest>::new("live"),
            current_stage: Id::<QuestStage>::new("s"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags,
        });

        let v = view(&p, &log);
        assert!(v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$quest_stage".to_string(),
            predicate: QuestFlagPredicate::IsSet,
        }));
        // Missing key → false.
        assert!(!v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$quest_other".to_string(),
            predicate: QuestFlagPredicate::IsSet,
        }));
    }

    #[test]
    fn quest_flag_greater_than_only_matches_numbers() {
        let p = player(&[]);
        let mut log = QuestLog::default();
        let mut flags = HashMap::new();
        flags.insert("$score".to_string(), YarnValue::Number(42.0));
        flags.insert("$name".to_string(), YarnValue::String("alice".to_string()));
        log.active.push(ActiveQuest {
            def_id: Id::<Quest>::new("live"),
            current_stage: Id::<QuestStage>::new("s"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags,
        });

        let v = view(&p, &log);
        // Numeric flag over threshold.
        assert!(v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$score".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(40.0),
        }));
        // Numeric flag under threshold.
        assert!(!v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$score".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(50.0),
        }));
        // String flag can't satisfy a numeric predicate.
        assert!(!v.evaluate(&ObjectiveCondition::QuestFlag {
            quest: Id::<Quest>::new("live"),
            key: "$name".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(0.0),
        }));
    }

    #[test]
    fn yarn_value_equals_rejects_cross_type() {
        // String to number → false, no coercion.
        assert!(!yarn_value_equals(
            &YarnValue::String("3".to_string()),
            &QuestFlagValue::Number(3.0),
        ));
        // Boolean to string → false.
        assert!(!yarn_value_equals(
            &YarnValue::Boolean(true),
            &QuestFlagValue::String("true".to_string()),
        ));
    }
}
