//! Five-level scale used across the game.

use serde::{Deserialize, Serialize};

/// Five-level scale used for danger ratings, loot quality, and
/// other graduated values.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize
)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Minimal.
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
