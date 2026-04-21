//! Pure [`ObjectiveCondition`] evaluation.
//!
//! Single free function over world state. Quest stages, triggers,
//! and anything else that needs a boolean over player + world
//! state all call [`evaluate`]. Adding a new condition variant
//! means adding one arm here.
//!
//! Callers in Bevy systems usually go through
//! [`QuestCtx::evaluate`](super::context::QuestCtx::evaluate)
//! rather than this free function — the ctx method assembles the
//! resource refs from its `SystemParam`. The free function exists
//! so unit tests (which can't construct a `SystemParam`) can still
//! drive the evaluator directly.

use bevy::log::warn;
use bevy_yarnspinner::prelude::YarnValue;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{
    ObjectiveCondition, Quest, QuestFlagPredicate, QuestFlagValue,
};

use super::refs::{PlayerView, SimView};
use super::state::QuestLog;

/// Evaluate a condition against live world state.
///
/// `stage_started_at` is the per-stage clock used by the `Wait`
/// variant; `None` signals the caller isn't inside a stage (trigger
/// eligibility, choice gate) in which case a `Wait` condition warns
/// and returns `false`.
pub fn evaluate(
    cond: &ObjectiveCondition,
    players: &PlayerView,
    sim: &SimView,
    stage_started_at: Option<GameTime>,
) -> bool {
    match cond {
        ObjectiveCondition::HaveItem(q) => {
            players.stash.has_item(&q.item, q.resolved_count(), q.scope)
        }
        ObjectiveCondition::HaveCredits(amount) => players.identity.credits.can_afford(*amount),
        ObjectiveCondition::FactionStanding {
            faction,
            min_standing,
        } => players.standings.standing(faction) >= *min_standing,
        ObjectiveCondition::HaveUpgrade(upgrade) => players.upgrades.has_upgrade(upgrade),
        ObjectiveCondition::HaveIntel(id) => players.intel.has(id),
        ObjectiveCondition::EventActive(event) => sim.events.iter().any(|e| &e.def_id == event),
        ObjectiveCondition::QuestActive(quest) => sim.quests.is_active(quest),
        ObjectiveCondition::QuestCompleted(quest) => sim.quests.is_completed_successfully(quest),
        ObjectiveCondition::QuestFlag {
            quest,
            key,
            predicate,
        } => evaluate_quest_flag(sim.quests, quest, key, predicate),
        ObjectiveCondition::NpcAlive(npc) => sim.registry.is_alive(npc),
        ObjectiveCondition::NpcDead(npc) => !sim.registry.is_alive(npc),
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
            sim.now.minutes_since(started_at) >= duration.minutes()
        }
        ObjectiveCondition::DaysWithoutPills { days } => sim.pills.days_without(sim.now) >= *days,
        ObjectiveCondition::DayReached { day } => sim.now.day.value() >= *day,
        ObjectiveCondition::DecisionEquals { decision, value } => {
            players.decisions.equals(decision, value)
        }
        ObjectiveCondition::AllOf(conds) => conds
            .iter()
            .all(|c| evaluate(c, players, sim, stage_started_at)),
        ObjectiveCondition::AnyOf(conds) => conds
            .iter()
            .any(|c| evaluate(c, players, sim, stage_started_at)),
        ObjectiveCondition::Not(inner) => !evaluate(inner, players, sim, stage_started_at),
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
