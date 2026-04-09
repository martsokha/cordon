//! Signed relationship value clamped to -100..=100, plus a
//! wider delta type used to shift it.
//!
//! [`Relation`] is the *absolute* value — a player's standing
//! with a faction, a min-standing threshold, a baseline between
//! two factions. It's an `i8` because its domain is bounded at
//! ±100 and it never needs to represent anything outside that
//! range.
//!
//! [`RelationDelta`] is the *shift* applied to a [`Relation`] —
//! a `StandingChange` consequence, a runtime nudge from a coup
//! event, etc. It's an `i16` so callers can express shifts that
//! overshoot the absolute range (`+200`, `-150`) without silent
//! clamping at the type boundary; the real clamp still happens
//! when the delta is folded into a [`Relation`] via
//! [`Relation::apply`].
//!
//! Keeping the two types distinct stops deltas and absolutes
//! from being accidentally confused — a common enough hazard
//! when both are the same primitive.

use serde::{Deserialize, Serialize};

/// An absolute relationship value on a -100 to +100 scale.
///
/// Used for faction standings, standing thresholds, and any
/// other "where does this relation sit right now" question.
/// Arithmetic via [`apply`](Self::apply) saturates at the
/// bounds.
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

    /// Apply an additive [`RelationDelta`], saturating at the
    /// bounds. The delta is wider than [`Relation`] so extreme
    /// shifts (`+200`) land correctly at `MAX` instead of
    /// wrapping through an intermediate `i8`.
    pub fn apply(&mut self, delta: RelationDelta) {
        let sum = self.0 as i32 + delta.0 as i32;
        self.0 = sum.clamp(Self::MIN.0 as i32, Self::MAX.0 as i32) as i8;
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

/// An additive shift applied to a [`Relation`].
///
/// Wider than `Relation` (`i16` vs `i8`) so authors can write
/// large one-off shifts without tripping serde range errors,
/// and so runtime math that scales by a float doesn't need to
/// clamp at two places. The final clamp happens when the
/// delta is folded into a `Relation` via [`Relation::apply`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct RelationDelta(i16);

impl RelationDelta {
    /// A zero shift.
    pub const ZERO: Self = Self(0);

    /// Create a new delta from a raw signed value.
    pub const fn new(value: i16) -> Self {
        Self(value)
    }

    /// Get the raw value.
    pub const fn value(self) -> i16 {
        self.0
    }
}

impl std::ops::Neg for RelationDelta {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_saturates_at_bounds() {
        let mut r = Relation::new(80);
        r.apply(RelationDelta::new(50));
        assert_eq!(r.value(), 100);

        let mut r = Relation::new(-80);
        r.apply(RelationDelta::new(-50));
        assert_eq!(r.value(), -100);
    }

    #[test]
    fn apply_wide_delta_does_not_wrap() {
        let mut r = Relation::NEUTRAL;
        r.apply(RelationDelta::new(500));
        assert_eq!(r.value(), 100);

        let mut r = Relation::NEUTRAL;
        r.apply(RelationDelta::new(-500));
        assert_eq!(r.value(), -100);
    }

    #[test]
    fn delta_negation() {
        let d = RelationDelta::new(15);
        assert_eq!((-d).value(), -15);
    }
}
