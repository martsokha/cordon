//! Sector definitions loaded from config.
//!
//! Each sector is a named area of the Zone with base danger, reward,
//! radio requirements, and travel time.

use serde::{Deserialize, Serialize};

use crate::primitive::id::Id;

/// A sector of the Zone, loaded from config.
///
/// Sectors define the static properties of a map area. Live state
/// (faction control, creature activity, etc.) is tracked separately
/// by the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorDef {
    /// Unique identifier (e.g., `"threshold"`, `"core"`).
    pub id: Id,
    /// Display name (e.g., `"The Threshold"`).
    pub name: String,
    /// Base danger level (0.0–1.0). Modified at runtime by events.
    pub base_danger: f32,
    /// Base reward quality (0.0–1.0). Affects loot table selection.
    pub base_reward: f32,
    /// Minimum radio upgrade level required to send runners here.
    pub radio_level_required: u8,
    /// How many days a round trip takes.
    pub travel_days: u32,
    /// Faction ID that controls this sector by default, if any.
    pub default_faction: Option<Id>,
}
