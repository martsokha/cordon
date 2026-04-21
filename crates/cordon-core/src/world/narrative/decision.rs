//! Player decisions — durable record of authored branching choices.
//!
//! Quests and events can record a decision via
//! [`Consequence::RecordDecision`](super::Consequence::RecordDecision)
//! and later gate content on it via
//! [`ObjectiveCondition::DecisionEquals`](super::ObjectiveCondition::DecisionEquals).
//!
//! A [`DecisionDef`] declares which values are legal for a given
//! decision. Load-time validation catches typos in quest JSON
//! (`record_decision` / `decision_equals` values that aren't listed
//! here) before the game starts.

use serde::{Deserialize, Serialize};

use crate::primitive::{Id, IdMarker};

/// Marker for decision definition IDs.
pub struct Decision;
impl IdMarker for Decision {}

/// A decision definition loaded from config.
///
/// Declares the set of legal values a decision can take. The id
/// doubles as the localization key for any UI that surfaces the
/// decision to the player (e.g., a decisions log).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDef {
    /// Unique identifier (e.g., `decision_garrison_support`).
    pub id: Id<Decision>,
    /// Legal values this decision can be recorded with. Recording
    /// or gating on a value not in this list is a load-time error.
    pub values: Vec<String>,
}
