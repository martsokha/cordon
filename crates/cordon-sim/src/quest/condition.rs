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
