//! NPC health value.

use serde::{Deserialize, Serialize};

/// Health of an NPC, in integer HP.
///
/// Drops from combat, radiation, and environmental hazards.
/// Recovers with medical supplies or rest. The default cap is 100;
/// relics or perks may raise the maximum on a per-NPC basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Health(u32);

impl Health {
    /// Default starting health (100 HP).
    pub const FULL: Self = Self(100);
    /// Dead.
    pub const ZERO: Self = Self(0);

    /// Create a new health value.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Get the raw HP value.
    pub fn value(self) -> u32 {
        self.0
    }

    /// Apply damage. Saturates at 0.
    pub fn damage(&mut self, amount: u32) {
        self.0 = self.0.saturating_sub(amount);
    }

    /// Apply healing, capped at the given maximum.
    pub fn heal(&mut self, amount: u32, max: u32) {
        self.0 = (self.0 + amount).min(max);
    }

    /// Whether the NPC is alive.
    pub fn is_alive(self) -> bool {
        self.0 > 0
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::FULL
    }
}
