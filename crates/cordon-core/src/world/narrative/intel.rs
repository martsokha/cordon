//! Intel definitions: data-driven pieces of information the player
//! discovers through events, quests, dialogue, and radio broadcasts.
//!
//! [`IntelDef`] is loaded from JSON config files. Title and description
//! are derived from the ID via localisation keys:
//! `intel.{id}.title` and `intel.{id}.description`.

use serde::{Deserialize, Serialize};

use crate::primitive::{Duration, Id, IdMarker};

/// Marker for intel definition IDs.
pub struct Intel;
impl IdMarker for Intel {}

/// Broad category for intel grouping and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntelCategory {
    /// Faction movements, alliances, betrayals.
    Faction,
    /// Hazard zones, anomaly shifts, weather.
    Environmental,
    /// Market shifts, supply disruptions, price intel.
    Economic,
    /// Hearsay, unverified tips, gossip.
    Rumour,
    /// Quest-related briefings and debriefings.
    Mission,
}

/// An intel definition loaded from config.
///
/// Title and description are localised: `intel.{id}.title` and
/// `intel.{id}.description`. No text fields here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelDef {
    /// Unique identifier and localisation key root.
    pub id: Id<Intel>,
    /// Display category for filtering and grouping.
    pub category: IntelCategory,
    /// How long this intel stays relevant after being granted.
    /// `None` means it never expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<Duration>,
}
