//! Pure [`ObjectiveCondition`] evaluation.
//!
//! Single-entry recursive evaluator over world state. Quest
//! stages, quest triggers, and anywhere else that needs a
//! boolean over player + world state all route through
//! [`evaluate`]. Extending the vocabulary means adding one arm
//! here and nothing else.

use bevy_yarnspinner::prelude::YarnValue;
use cordon_core::entity::player::PlayerState;
use cordon_core::world::event::ActiveEvent;
use cordon_core::world::narrative::consequence::ObjectiveCondition;

use super::state::QuestLog;

/// Live world state the evaluator reads from. Kept as a single
/// struct so the evaluator has one clean parameter instead of a
/// growing tuple of references.
pub struct WorldView<'a> {
    pub player: &'a PlayerState,
    pub events: &'a [ActiveEvent],
    pub quests: &'a QuestLog,
}

/// Evaluate a condition against the given world view.
///
/// Recursive for [`AllOf`](ObjectiveCondition::AllOf),
/// [`AnyOf`](ObjectiveCondition::AnyOf), and
/// [`Not`](ObjectiveCondition::Not). Leaf conditions do simple
/// lookups against the player state, event log, or quest log.
pub fn evaluate(cond: &ObjectiveCondition, world: &WorldView<'_>) -> bool {
    match cond {
        ObjectiveCondition::HaveItem { item, count, scope } => {
            world.player.has_item(item, *count, *scope)
        }

        ObjectiveCondition::HaveCredits(amount) => world.player.credits.can_afford(*amount),

        ObjectiveCondition::FactionStanding {
            faction,
            min_standing,
        } => world.player.standing(faction) >= *min_standing,

        ObjectiveCondition::HaveUpgrade(upgrade) => world.player.has_upgrade(upgrade),

        ObjectiveCondition::EventActive(event) => world.events.iter().any(|e| &e.def_id == event),

        ObjectiveCondition::QuestActive(quest) => world.quests.is_active(quest),

        ObjectiveCondition::QuestCompleted(quest) => world.quests.is_completed_successfully(quest),

        ObjectiveCondition::QuestFlag { quest, key, equals } => {
            match world.quests.active_instance(quest) {
                Some(active) => match active.flags.get(key) {
                    Some(value) => yarn_value_matches(value, equals),
                    None => false,
                },
                // Also check completed quests — flag reads on finished
                // quests are the main way later quests branch on
                // earlier outcomes.
                None => world
                    .quests
                    .completed
                    .iter()
                    .rev()
                    .find(|c| &c.def_id == quest)
                    .and_then(|c| c.flags.get(key))
                    .map(|value| yarn_value_matches(value, equals))
                    .unwrap_or(false),
            }
        }

        ObjectiveCondition::Wait => true,

        ObjectiveCondition::AllOf(conds) => conds.iter().all(|c| evaluate(c, world)),
        ObjectiveCondition::AnyOf(conds) => conds.iter().any(|c| evaluate(c, world)),
        ObjectiveCondition::Not(inner) => !evaluate(inner, world),
    }
}

/// Compare a [`YarnValue`] flag to a string literal from a
/// [`ObjectiveCondition::QuestFlag`] check, following Yarn's
/// loose casting rules. String flags compare textually; numeric
/// flags are parsed from the literal; boolean flags accept
/// `"true"` / `"false"` case-insensitively.
fn yarn_value_matches(value: &YarnValue, expected: &str) -> bool {
    match value {
        YarnValue::String(s) => s == expected,
        YarnValue::Number(n) => expected.parse::<f32>().map(|e| *n == e).unwrap_or(false),
        YarnValue::Boolean(b) => match expected.to_ascii_lowercase().as_str() {
            "true" => *b,
            "false" => !*b,
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy_yarnspinner::prelude::YarnValue;
    use cordon_core::entity::faction::Faction;
    use cordon_core::entity::player::PlayerState;
    use cordon_core::primitive::{Credits, GameTime, Id, Relation};
    use cordon_core::world::narrative::consequence::ObjectiveCondition;
    use cordon_core::world::narrative::quest::{Quest, QuestStage};

    use super::{WorldView, evaluate, yarn_value_matches};
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
        }
    }

    #[test]
    fn have_credits_threshold() {
        let mut p = player(&[]);
        p.credits = Credits::new(500);
        let log = QuestLog::default();
        assert!(evaluate(
            &ObjectiveCondition::HaveCredits(Credits::new(500)),
            &view(&p, &log)
        ));
        assert!(evaluate(
            &ObjectiveCondition::HaveCredits(Credits::new(499)),
            &view(&p, &log)
        ));
        assert!(!evaluate(
            &ObjectiveCondition::HaveCredits(Credits::new(501)),
            &view(&p, &log)
        ));
    }

    #[test]
    fn faction_standing_at_threshold() {
        let mut p = player(&["garrison"]);
        if let Some(s) = p.standing_mut(&Id::<Faction>::new("garrison")) {
            s.apply(Relation::new(50));
        }
        let log = QuestLog::default();
        let cond = ObjectiveCondition::FactionStanding {
            faction: Id::<Faction>::new("garrison"),
            min_standing: Relation::new(50),
        };
        assert!(evaluate(&cond, &view(&p, &log)));
        let cond_too_high = ObjectiveCondition::FactionStanding {
            faction: Id::<Faction>::new("garrison"),
            min_standing: Relation::new(60),
        };
        assert!(!evaluate(&cond_too_high, &view(&p, &log)));
    }

    #[test]
    fn all_of_short_circuits_on_false() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::AllOf(vec![
            ObjectiveCondition::Wait,
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
        ]);
        assert!(!evaluate(&cond, &view(&p, &log)));
    }

    #[test]
    fn any_of_short_circuits_on_true() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::AnyOf(vec![
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
            ObjectiveCondition::Wait,
        ]);
        assert!(evaluate(&cond, &view(&p, &log)));
    }

    #[test]
    fn not_flips_result() {
        let p = player(&[]);
        let log = QuestLog::default();
        let cond = ObjectiveCondition::Not(Box::new(ObjectiveCondition::Wait));
        assert!(!evaluate(&cond, &view(&p, &log)));
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

        assert!(evaluate(
            &ObjectiveCondition::QuestActive(Id::<Quest>::new("mainline")),
            &view(&p, &log)
        ));
        assert!(!evaluate(
            &ObjectiveCondition::QuestActive(Id::<Quest>::new("sidequest")),
            &view(&p, &log)
        ));
    }

    #[test]
    fn quest_flag_reads_active_first_then_completed() {
        let p = player(&[]);
        let mut log = QuestLog::default();

        // Active quest with a flag.
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

        // Completed quest with a different flag value.
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

        // Active quest hit.
        assert!(evaluate(
            &ObjectiveCondition::QuestFlag {
                quest: Id::<Quest>::new("live"),
                key: "$quest_choice".to_string(),
                equals: "accepted".to_string(),
            },
            &view(&p, &log)
        ));
        // Completed quest hit — active_instance miss falls
        // through to the completed scan.
        assert!(evaluate(
            &ObjectiveCondition::QuestFlag {
                quest: Id::<Quest>::new("done"),
                key: "$quest_choice".to_string(),
                equals: "refused".to_string(),
            },
            &view(&p, &log)
        ));
        // Wrong expected value.
        assert!(!evaluate(
            &ObjectiveCondition::QuestFlag {
                quest: Id::<Quest>::new("live"),
                key: "$quest_choice".to_string(),
                equals: "refused".to_string(),
            },
            &view(&p, &log)
        ));
        // Unknown key.
        assert!(!evaluate(
            &ObjectiveCondition::QuestFlag {
                quest: Id::<Quest>::new("live"),
                key: "$quest_missing".to_string(),
                equals: "anything".to_string(),
            },
            &view(&p, &log)
        ));
    }

    #[test]
    fn yarn_value_number_non_numeric_expected_fails_gracefully() {
        // The load-bearing behaviour here is our `.unwrap_or(false)`
        // — passing a non-numeric literal to a numeric flag must
        // return false instead of bubbling up a parse error.
        assert!(!yarn_value_matches(&YarnValue::Number(3.0), "three"));
    }

    #[test]
    fn yarn_value_bool_rejects_non_truthy_strings() {
        // Our match arm only accepts the literals "true" /
        // "false" (case-insensitive) and rejects everything
        // else — including other truthy-looking strings like
        // "1" or "yes". Lock that in.
        assert!(!yarn_value_matches(&YarnValue::Boolean(true), "1"));
        assert!(!yarn_value_matches(&YarnValue::Boolean(true), "yes"));
        assert!(!yarn_value_matches(&YarnValue::Boolean(true), ""));
    }
}
