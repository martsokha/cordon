//! Quest flag values and predicates.
//!
//! Quest flags are typed key-value pairs written by Yarn when a
//! `Talk` stage finishes and read by later stages via
//! [`ObjectiveCondition::QuestFlag`](super::ObjectiveCondition::QuestFlag).
//! Authoring targets a flag like
//! `{ "quest": "q", "key": "$quest_choice", "predicate": { "equals": "accept" } }`.
//!
//! Two types live here:
//!
//! - [`QuestFlagValue`] — a serde-friendly mirror of the three
//!   YarnValue variants (number, string, boolean). Exists because
//!   cordon-core must stay independent of `bevy_yarnspinner`; the
//!   sim side converts at the evaluator boundary.
//!
//! - [`QuestFlagPredicate`] — how the author wants to compare
//!   the live flag to something. More than equality: `IsSet`,
//!   numeric comparisons, and negation.
//!
//! Coercion rules at the evaluator (defined in cordon-sim) follow
//! Yarn's loose casting: strings compare textually, numbers parse
//! the comparand, booleans accept `true` / `false` (case-insensitive).

use serde::{Deserialize, Serialize};

/// Serde-friendly mirror of the three [`bevy_yarnspinner::prelude::YarnValue`]
/// variants. Not a direct alias because cordon-core must not
/// depend on the Yarn runtime; the sim side converts on each
/// evaluation.
#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuestFlagValue {
    /// A numeric flag. Matches `YarnValue::Number`.
    Number(f32),
    /// A boolean flag. Matches `YarnValue::Boolean`. Must come
    /// before `String` in the untagged enum so JSON `true` /
    /// `false` literals parse as booleans, not as strings.
    Boolean(bool),
    /// A string flag. Matches `YarnValue::String`. Catches any
    /// JSON input that isn't a number or boolean.
    String(String),
}

/// How a [`QuestFlagPredicate`] matches a live flag against the
/// authored value.
#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestFlagPredicate {
    /// Flag must equal this value. String equality is textual;
    /// numeric equality uses `==` on `f32`; boolean equality is
    /// straight comparison.
    Equals(QuestFlagValue),
    /// Flag must NOT equal this value.
    NotEquals(QuestFlagValue),
    /// Flag is numeric and strictly greater than the given
    /// threshold. String / boolean flags fail this predicate.
    GreaterThan(f32),
    /// Flag is numeric and strictly less than the given threshold.
    LessThan(f32),
    /// Flag is present in the active quest's flag bag, regardless
    /// of its value. Useful for "the player made *some* choice".
    IsSet,
}
