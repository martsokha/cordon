//! Perk definitions for NPCs.
//!
//! Perks are hidden NPC traits revealed through gameplay actions.
//! Each perk has a unique ID and affects runner missions or guard duty.

use serde::{Deserialize, Serialize};

use crate::primitive::id::{Id, IdMarker};

/// Marker for perk IDs.
pub struct Perk;
impl IdMarker for Perk {}

/// Polarity of a perk's effect on the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerkPolarity {
    /// Beneficial to the player (e.g., Scavenger's Eye, Hard to Kill).
    Positive,
    /// Harmful to the player (e.g., Coward, Sticky Fingers).
    Negative,
    /// Unpredictable or mixed (e.g., Lucky).
    Neutral,
}

/// Perk definition loaded from config.
///
/// Perks are hidden NPC traits revealed through gameplay actions.
/// Each perk has a unique ID and affects runner missions or guard duty.
/// The [`id`](PerkDef::id) doubles as the localization key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerkDef {
    /// Unique identifier and localization key (e.g., `"scavengers_eye"`).
    pub id: Id<Perk>,
    /// Whether this perk helps, hurts, or is unpredictable.
    pub polarity: PerkPolarity,
}
