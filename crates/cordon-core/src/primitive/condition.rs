//! Item and equipment condition value.

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// Condition of an item or piece of equipment, from 0.0 (destroyed) to 1.0 (factory new).
///
/// Price scales with condition² — a 0.5 condition item is worth ~25% of
/// base price. Condition degrades with use and over time in poor storage.
/// Repairs are done by sending items to faction workshops.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Display)]
#[display("{_0:.0}%", _0 = _0 * 100.0)]
pub struct Condition(f32);

impl Condition {
    /// Destroyed (0.0).
    pub const ZERO: Self = Self(0.0);

    /// Factory new (1.0).
    pub const PERFECT: Self = Self(1.0);

    /// Create a new condition value, clamped to 0.0–1.0.
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw float value.
    pub fn value(self) -> f32 {
        self.0
    }

    /// The condition² factor used in price calculations.
    pub fn price_factor(self) -> f32 {
        self.0 * self.0
    }

    /// Apply wear (subtract), clamping at 0.0.
    pub fn degrade(&mut self, amount: f32) {
        self.0 = (self.0 - amount).max(0.0);
    }

    /// Apply repair (add), clamping at 1.0.
    pub fn repair(&mut self, amount: f32) {
        self.0 = (self.0 + amount).min(1.0);
    }
}
