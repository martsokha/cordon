//! Item condition expressed as a 0.0–1.0 ratio of current/max durability.

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Condition of an item or piece of equipment, from 0.0 (destroyed) to
/// 1.0 (factory new). Computed from `current_durability / max_durability`.
///
/// Condition is a *derived view*, not stored data. The authoritative
/// state lives in [`ItemInstance::durability`](crate::item::ItemInstance::durability).
/// This wrapper exists so the price-calculation code and UI can speak
/// in fractional terms when they need to.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize, Display)]
#[display("{_0:.0}%", _0 = _0 * 100.0)]
pub struct Condition(f32);

impl Condition {
    /// Factory new (1.0).
    pub const PERFECT: Self = Self(1.0);
    /// Destroyed (0.0).
    pub const ZERO: Self = Self(0.0);

    /// Create a new condition value, clamped to 0.0–1.0.
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Compute condition from current and max durability values.
    /// A `max` of zero is treated as "indestructible", returning [`Condition::PERFECT`].
    pub fn from_durability(current: u32, max: u32) -> Self {
        if max == 0 {
            return Self::PERFECT;
        }
        Self::new(current as f32 / max as f32)
    }

    /// Get the raw float value.
    pub fn value(self) -> f32 {
        self.0
    }

    /// The condition² factor used in price calculations.
    pub fn price_factor(self) -> f32 {
        self.0 * self.0
    }
}
