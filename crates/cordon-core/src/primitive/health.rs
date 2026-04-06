//! NPC health value.

use serde::{Deserialize, Serialize};

/// Health of an NPC, from 0.0 (dead) to 1.0 (full health).
///
/// Drops from combat, radiation, and environmental hazards.
/// Recovers with medical supplies or rest.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Health(f32);

impl Health {
    /// Full health (1.0).
    pub const FULL: Self = Self(1.0);
    /// Dead (0.0).
    pub const ZERO: Self = Self(0.0);

    /// Create a new health value, clamped to 0.0–1.0.
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw float value.
    pub fn value(self) -> f32 {
        self.0
    }

    /// Apply damage, clamping at 0.0.
    pub fn damage(&mut self, amount: f32) {
        self.0 = (self.0 - amount).max(0.0);
    }

    /// Apply healing, clamping at 1.0.
    pub fn heal(&mut self, amount: f32) {
        self.0 = (self.0 + amount).min(1.0);
    }

    /// Whether the NPC is alive.
    pub fn is_alive(self) -> bool {
        self.0 > 0.0
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::FULL
    }
}
