//! Five-level scale used across the game.

use serde::{Deserialize, Serialize};

/// Five-level scale used for danger ratings, loot quality, and
/// other graduated values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Minimal.
    #[default]
    VeryLow,
    /// Below average.
    Low,
    /// Moderate.
    Medium,
    /// Significant.
    High,
    /// Extreme.
    VeryHigh,
}
