//! Faction definitions and the standing system.
//!
//! [`FactionDef`] is loaded from config. [`Standing`] tracks the
//! player's relationship with a faction on a -100 to +100 scale.

use serde::{Deserialize, Serialize};

use crate::object::id::Id;

/// Faction definition loaded from config.
///
/// Each faction has a unique ID, a name, and properties that determine
/// how it interacts with the player and other factions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDef {
    /// Unique identifier (e.g., `"order"`, `"drifters"`).
    pub id: Id,
    /// Display name (e.g., `"The Order"`).
    pub name: String,
    /// The faction's core belief or motivation.
    pub philosophy: String,
    /// How the faction is organized.
    pub structure: String,
    /// Whether NPCs from this faction can be recruited as runners/guards.
    pub recruitable: bool,
    /// Rank titles for tiers 1–5 (e.g., `["Grunt", "Soldier", ...]`).
    pub rank_titles: [String; 5],
    /// Item kind names this faction typically buys.
    pub buys: Vec<String>,
    /// Item kind names this faction typically sells.
    pub sells: Vec<String>,
    /// Base relations with other factions: `(faction_id, initial_standing)`.
    pub relations: Vec<(Id, i8)>,
}

/// A faction standing value, clamped to -100..=100.
///
/// Standings determine how a faction treats the player:
/// - -100 to -50: Hostile (kill on sight, raids, embargoes)
/// - -49 to -1: Unfriendly (bad prices, threats)
/// - 0 to 49: Neutral (default trade)
/// - 50 to 79: Friendly (good prices, intel, protection)
/// - 80 to 100: Allied (best prices, exclusive missions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing(i8);

impl Standing {
    /// Minimum possible standing.
    pub const MIN: i8 = -100;
    /// Maximum possible standing.
    pub const MAX: i8 = 100;

    /// Create a new standing, clamped to the valid range.
    pub fn new(value: i8) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    /// Create a neutral standing (0).
    pub fn neutral() -> Self {
        Self(0)
    }

    /// Get the raw standing value.
    pub fn value(self) -> i8 {
        self.0
    }

    /// Apply a delta to the standing, clamping to valid range.
    pub fn apply(&mut self, delta: i8) {
        self.0 = (self.0 as i16 + delta as i16).clamp(Self::MIN as i16, Self::MAX as i16) as i8;
    }

    /// Standing is -50 or below.
    pub fn is_hostile(self) -> bool {
        self.0 <= -50
    }

    /// Standing is between -49 and -1.
    pub fn is_unfriendly(self) -> bool {
        self.0 > -50 && self.0 < 0
    }

    /// Standing is between 0 and 49.
    pub fn is_neutral(self) -> bool {
        self.0 >= 0 && self.0 < 50
    }

    /// Standing is between 50 and 79.
    pub fn is_friendly(self) -> bool {
        self.0 >= 50 && self.0 < 80
    }

    /// Standing is 80 or above.
    pub fn is_allied(self) -> bool {
        self.0 >= 80
    }
}

impl Default for Standing {
    fn default() -> Self {
        Self::neutral()
    }
}
