//! Pure [`ObjectiveCondition`] evaluation.
//!
//! Single free function over world state. Quest stages, triggers,
//! and anything else that needs a boolean over player + world
//! state all call [`evaluate`]. Adding a new condition variant
//! means adding one arm here.

use bevy::log::warn;
use bevy_yarnspinner::prelude::YarnValue;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{
    ActiveEvent, ObjectiveCondition, Quest, QuestFlagPredicate, QuestFlagValue,
};

use super::registry::TemplateRegistry;
use super::state::QuestLog;
use crate::resources::{PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash, PlayerUpgrades};

/// Evaluate a condition against live world state.
pub fn evaluate(
    cond: &ObjectiveCondition,
    identity: &PlayerIdentity,
    standings: &PlayerStandings,
    upgrades: &PlayerUpgrades,
    stash: &PlayerStash,
    intel: &PlayerIntel,
    events: &[ActiveEvent],
    quests: &QuestLog,
    registry: &TemplateRegistry,
    now: GameTime,
    stage_started_at: Option<GameTime>,
) -> bool {
    match cond {
        ObjectiveCondition::HaveItem(q) => stash.has_item(&q.item, q.resolved_count(), q.scope),
        ObjectiveCondition::HaveCredits(amount) => identity.credits.can_afford(*amount),
        ObjectiveCondition::FactionStanding {
            faction,
            min_standing,
        } => standings.standing(faction) >= *min_standing,
        ObjectiveCondition::HaveUpgrade(upgrade) => upgrades.has_upgrade(upgrade),
        ObjectiveCondition::HaveIntel(id) => intel.has(id),
        ObjectiveCondition::EventActive(event) => events.iter().any(|e| &e.def_id == event),
        ObjectiveCondition::QuestActive(quest) => quests.is_active(quest),
        ObjectiveCondition::QuestCompleted(quest) => quests.is_completed_successfully(quest),
        ObjectiveCondition::QuestFlag {
            quest,
            key,
            predicate,
        } => evaluate_quest_flag(quests, quest, key, predicate),
        ObjectiveCondition::NpcAlive(npc) => registry.is_alive(npc),
        ObjectiveCondition::NpcDead(npc) => !registry.is_alive(npc),
        ObjectiveCondition::NpcAtLocation { npc, area } => {
            warn!(
                "NpcAtLocation({}, {}) — not wired, returning false",
                npc.as_str(),
                area.as_str()
            );
            false
        }
        ObjectiveCondition::Wait { duration } => {
            let Some(started_at) = stage_started_at else {
                warn!("Wait condition without stage clock — returning false");
                return false;
            };
            now.minutes_since(started_at) >= duration.minutes()
        }
        ObjectiveCondition::AllOf(conds) => conds.iter().all(|c| {
            evaluate(
                c,
                identity,
                standings,
                upgrades,
                stash,
                intel,
                events,
                quests,
                registry,
                now,
                stage_started_at,
            )
        }),
        ObjectiveCondition::AnyOf(conds) => conds.iter().any(|c| {
            evaluate(
                c,
                identity,
                standings,
                upgrades,
                stash,
                intel,
                events,
                quests,
                registry,
                now,
                stage_started_at,
            )
        }),
        ObjectiveCondition::Not(inner) => !evaluate(
            inner,
            identity,
            standings,
            upgrades,
            stash,
            intel,
            events,
            quests,
            registry,
            now,
            stage_started_at,
        ),
    }
}

fn evaluate_quest_flag(
    quests: &QuestLog,
    quest: &Id<Quest>,
    key: &str,
    predicate: &QuestFlagPredicate,
) -> bool {
    let value = quests
        .active_instance(quest)
        .and_then(|a| a.flags.get(key))
        .or_else(|| {
            quests
                .completed
                .iter()
                .rev()
                .find(|c| &c.def_id == quest)
                .and_then(|c| c.flags.get(key))
        });
    match (predicate, value) {
        (QuestFlagPredicate::IsSet, v) => v.is_some(),
        (_, None) => false,
        (QuestFlagPredicate::Equals(expected), Some(v)) => yarn_value_equals(v, expected),
        (QuestFlagPredicate::NotEquals(expected), Some(v)) => !yarn_value_equals(v, expected),
        (QuestFlagPredicate::GreaterThan(threshold), Some(v)) => {
            yarn_value_as_number(v).is_some_and(|n| n > *threshold)
        }
        (QuestFlagPredicate::LessThan(threshold), Some(v)) => {
            yarn_value_as_number(v).is_some_and(|n| n < *threshold)
        }
    }
}

fn yarn_value_equals(value: &YarnValue, expected: &QuestFlagValue) -> bool {
    match (value, expected) {
        (YarnValue::String(a), QuestFlagValue::String(b)) => a == b,
        (YarnValue::Number(a), QuestFlagValue::Number(b)) => a == b,
        (YarnValue::Boolean(a), QuestFlagValue::Boolean(b)) => a == b,
        _ => false,
    }
}

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

    use super::{evaluate, yarn_value_equals};
    use crate::quest::registry::TemplateRegistry;
    use crate::quest::state::{ActiveQuest, CompletedQuest, QuestLog};
    use crate::resources::{
        PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash, PlayerUpgrades,
    };

    struct Ctx {
        identity: PlayerIdentity,
        standings: PlayerStandings,
        upgrades: PlayerUpgrades,
        stash: PlayerStash,
        intel: PlayerIntel,
        log: QuestLog,
        registry: TemplateRegistry,
    }

    impl Ctx {
        fn new(factions: &[&str]) -> Self {
            let ids: Vec<Id<Faction>> = factions.iter().map(|f| Id::new(*f)).collect();
            let state = PlayerState::new(&ids);
            Self {
                identity: PlayerIdentity {
                    xp: state.xp,
                    credits: state.credits,
                    debt: state.debt,
                },
                standings: PlayerStandings {
                    standings: state.standings,
                },
                upgrades: PlayerUpgrades {
                    upgrades: state.upgrades,
                },
                stash: PlayerStash {
                    pending_items: state.pending_items,
                    hidden_storage: state.hidden_storage,
                },
                intel: PlayerIntel::default(),
                log: QuestLog::default(),
                registry: TemplateRegistry::default(),
            }
        }

        fn eval(&self, cond: &ObjectiveCondition) -> bool {
            evaluate(
                cond,
                &self.identity,
                &self.standings,
                &self.upgrades,
                &self.stash,
                &self.intel,
                &[],
                &self.log,
                &self.registry,
                GameTime::new(),
                None,
            )
        }

        fn eval_with_clock(
            &self,
            cond: &ObjectiveCondition,
            now: GameTime,
            stage_started_at: GameTime,
        ) -> bool {
            evaluate(
                cond,
                &self.identity,
                &self.standings,
                &self.upgrades,
                &self.stash,
                &self.intel,
                &[],
                &self.log,
                &self.registry,
                now,
                Some(stage_started_at),
            )
        }
    }

    #[test]
    fn have_credits_threshold() {
        let mut ctx = Ctx::new(&[]);
        ctx.identity.credits = Credits::new(500);
        assert!(ctx.eval(&ObjectiveCondition::HaveCredits(Credits::new(500))));
        assert!(ctx.eval(&ObjectiveCondition::HaveCredits(Credits::new(499))));
        assert!(!ctx.eval(&ObjectiveCondition::HaveCredits(Credits::new(501))));
    }

    #[test]
    fn faction_standing_at_threshold() {
        let mut ctx = Ctx::new(&["garrison"]);
        if let Some(s) = ctx.standings.standing_mut(&Id::<Faction>::new("garrison")) {
            s.apply(RelationDelta::new(50));
        }
        assert!(ctx.eval(&ObjectiveCondition::FactionStanding {
            faction: Id::new("garrison"),
            min_standing: Relation::new(50),
        }));
        assert!(!ctx.eval(&ObjectiveCondition::FactionStanding {
            faction: Id::new("garrison"),
            min_standing: Relation::new(60),
        }));
    }

    #[test]
    fn all_of_short_circuits_on_false() {
        let ctx = Ctx::new(&[]);
        let cond = ObjectiveCondition::AllOf(vec![
            ObjectiveCondition::Wait {
                duration: Duration::INSTANT,
            },
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
        ]);
        assert!(!ctx.eval_with_clock(&cond, GameTime::new(), GameTime::new()));
    }

    #[test]
    fn any_of_short_circuits_on_true() {
        let ctx = Ctx::new(&[]);
        let cond = ObjectiveCondition::AnyOf(vec![
            ObjectiveCondition::HaveCredits(Credits::new(9999)),
            ObjectiveCondition::Wait {
                duration: Duration::INSTANT,
            },
        ]);
        assert!(ctx.eval_with_clock(&cond, GameTime::new(), GameTime::new()));
    }

    #[test]
    fn not_flips_result() {
        let ctx = Ctx::new(&[]);
        let cond = ObjectiveCondition::Not(Box::new(ObjectiveCondition::Wait {
            duration: Duration::INSTANT,
        }));
        assert!(!ctx.eval_with_clock(&cond, GameTime::new(), GameTime::new()));
    }

    #[test]
    fn wait_without_stage_clock_returns_false() {
        let ctx = Ctx::new(&[]);
        assert!(!ctx.eval(&ObjectiveCondition::Wait {
            duration: Duration::INSTANT,
        }));
    }

    #[test]
    fn wait_honours_duration() {
        let ctx = Ctx::new(&[]);
        let cond = ObjectiveCondition::Wait {
            duration: Duration::from_minutes(30),
        };
        let start = GameTime::new();
        assert!(!ctx.eval_with_clock(&cond, start, start));
        let mut now_late = start;
        now_late.advance_minutes(30);
        assert!(ctx.eval_with_clock(&cond, now_late, start));
    }

    #[test]
    fn quest_active_lookup() {
        let mut ctx = Ctx::new(&[]);
        ctx.log.active.push(ActiveQuest {
            def_id: Id::new("mainline"),
            current_stage: Id::<QuestStage>::new("intro"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags: HashMap::new(),
        });
        assert!(ctx.eval(&ObjectiveCondition::QuestActive(Id::new("mainline"))));
        assert!(!ctx.eval(&ObjectiveCondition::QuestActive(Id::new("sidequest"))));
    }

    #[test]
    fn quest_flag_equals_active_first_then_completed() {
        let mut ctx = Ctx::new(&[]);
        let mut flags = HashMap::new();
        flags.insert(
            "$quest_choice".to_string(),
            YarnValue::String("accepted".to_string()),
        );
        ctx.log.active.push(ActiveQuest {
            def_id: Id::new("live"),
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
        ctx.log.completed.push(CompletedQuest {
            def_id: Id::new("done"),
            started_at: GameTime::new(),
            completed_at: GameTime::new(),
            success: false,
            outcome_stage: Id::<QuestStage>::new("outcome_refuse"),
            flags: completed_flags,
        });
        assert!(ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$quest_choice".to_string(),
            predicate: QuestFlagPredicate::Equals(QuestFlagValue::String("accepted".to_string())),
        }));
        assert!(ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("done"),
            key: "$quest_choice".to_string(),
            predicate: QuestFlagPredicate::Equals(QuestFlagValue::String("refused".to_string())),
        }));
    }

    #[test]
    fn quest_flag_is_set_matches_any_value() {
        let mut ctx = Ctx::new(&[]);
        let mut flags = HashMap::new();
        flags.insert("$quest_stage".to_string(), YarnValue::Number(3.0));
        ctx.log.active.push(ActiveQuest {
            def_id: Id::new("live"),
            current_stage: Id::<QuestStage>::new("s"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags,
        });
        assert!(ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$quest_stage".to_string(),
            predicate: QuestFlagPredicate::IsSet,
        }));
        assert!(!ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$quest_other".to_string(),
            predicate: QuestFlagPredicate::IsSet,
        }));
    }

    #[test]
    fn quest_flag_greater_than_only_matches_numbers() {
        let mut ctx = Ctx::new(&[]);
        let mut flags = HashMap::new();
        flags.insert("$score".to_string(), YarnValue::Number(42.0));
        flags.insert("$name".to_string(), YarnValue::String("alice".to_string()));
        ctx.log.active.push(ActiveQuest {
            def_id: Id::new("live"),
            current_stage: Id::<QuestStage>::new("s"),
            started_at: GameTime::new(),
            stage_started_at: GameTime::new(),
            flags,
        });
        assert!(ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$score".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(40.0),
        }));
        assert!(!ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$score".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(50.0),
        }));
        assert!(!ctx.eval(&ObjectiveCondition::QuestFlag {
            quest: Id::new("live"),
            key: "$name".to_string(),
            predicate: QuestFlagPredicate::GreaterThan(0.0),
        }));
    }

    #[test]
    fn yarn_value_equals_rejects_cross_type() {
        assert!(!yarn_value_equals(
            &YarnValue::String("3".to_string()),
            &QuestFlagValue::Number(3.0),
        ));
        assert!(!yarn_value_equals(
            &YarnValue::Boolean(true),
            &QuestFlagValue::String("true".to_string()),
        ));
    }
}
