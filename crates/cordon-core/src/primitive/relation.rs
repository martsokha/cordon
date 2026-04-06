//! Signed relationship value clamped to -100..=100.

use serde::{Deserialize, Serialize};

/// A relationship value on a -100 to +100 scale.
///
/// Used for faction standings, standing thresholds, and standing
/// deltas. Arithmetic saturates at the bounds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Relation(i8);

impl Relation {
    /// Maximum relation value.
    pub const MAX: Self = Self(100);
    /// Minimum relation value.
    pub const MIN: Self = Self(-100);
    /// Neutral relation (0).
    pub const NEUTRAL: Self = Self(0);

    /// Create a new relation, clamped to the valid range.
    pub fn new(value: i8) -> Self {
        Self(value.clamp(Self::MIN.0, Self::MAX.0))
    }

    /// Get the raw value.
    pub fn value(self) -> i8 {
        self.0
    }

    /// Apply an additive delta, saturating at the bounds.
    pub fn apply(&mut self, delta: Relation) {
        self.0 =
            (self.0 as i16 + delta.0 as i16).clamp(Self::MIN.0 as i16, Self::MAX.0 as i16) as i8;
    }

    /// Relation is -50 or below.
    pub fn is_hostile(self) -> bool {
        self.0 <= -50
    }

    /// Relation is between -49 and -1.
    pub fn is_unfriendly(self) -> bool {
        self.0 > -50 && self.0 < 0
    }

    /// Relation is between 0 and 49.
    pub fn is_neutral(self) -> bool {
        self.0 >= 0 && self.0 < 50
    }

    /// Relation is between 50 and 79.
    pub fn is_friendly(self) -> bool {
        self.0 >= 50 && self.0 < 80
    }

    /// Relation is 80 or above.
    pub fn is_allied(self) -> bool {
        self.0 >= 80
    }
}
