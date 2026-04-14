//! In-game currency.

use std::ops::{Add, AddAssign, Sub, SubAssign};

use bevy::prelude::Component;
use derive_more::Display;
use serde::{Deserialize, Serialize};

/// The Zone's currency. Used for trading, bribes, upgrades, and payroll.
///
/// Wraps a `u32`. Cannot go negative — subtraction saturates at zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Component, Display, Serialize, Deserialize)]
#[display("{_0} cr")]
pub struct Credits(u32);

impl Credits {
    /// Zero credits.
    pub const ZERO: Self = Self(0);

    /// Create from a raw value.
    pub fn new(amount: u32) -> Self {
        Self(amount)
    }

    /// Get the raw value.
    pub fn value(self) -> u32 {
        self.0
    }

    /// Whether this amount is sufficient to cover a cost.
    pub fn can_afford(self, cost: Credits) -> bool {
        self.0 >= cost.0
    }
}

impl Add for Credits {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Credits {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Credits {
    type Output = Self;

    /// Saturates at zero — never goes negative.
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Credits {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl From<u32> for Credits {
    fn from(amount: u32) -> Self {
        Self(amount)
    }
}
