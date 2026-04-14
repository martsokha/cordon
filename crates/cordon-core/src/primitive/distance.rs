//! Distance between two points in map units.

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// A distance in map units. Always non-negative.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Display
)]
#[display("{_0:.1}m")]
pub struct Distance(f32);

impl Distance {
    /// Zero distance.
    pub const ZERO: Self = Self(0.0);

    /// Create a new distance. Negative values are clamped to zero.
    pub fn new(value: f32) -> Self {
        Self(value.max(0.0))
    }

    /// Get the raw value in map units.
    pub fn value(self) -> f32 {
        self.0
    }
}
