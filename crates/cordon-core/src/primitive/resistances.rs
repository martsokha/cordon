//! Protection values against all damage types.

use serde::{Deserialize, Serialize};

/// Protection against all damage types.
///
/// Each value is an absolute protection rating. Compared directly
/// against the corresponding threat value (ammo penetration, hazard
/// intensity, radiation level). Higher = more protection.
///
/// Used by armor, consumable buffs, and relics.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resistances {
    /// Ballistic protection (vs ammo penetration).
    pub ballistic: u32,
    /// Radiation protection.
    pub radiation: u32,
    /// Chemical hazard protection.
    pub chemical: u32,
    /// Thermal hazard protection.
    pub thermal: u32,
    /// Electric hazard protection.
    pub electric: u32,
    /// Gravitational anomaly protection.
    pub gravitational: u32,
}

impl Resistances {
    /// No protection.
    pub const NONE: Self = Self {
        ballistic: 0,
        radiation: 0,
        chemical: 0,
        thermal: 0,
        electric: 0,
        gravitational: 0,
    };

    /// Combine two resistance sets (e.g., suit + helmet).
    pub fn combine(self, other: Self) -> Self {
        Self {
            ballistic: self.ballistic + other.ballistic,
            radiation: self.radiation + other.radiation,
            chemical: self.chemical + other.chemical,
            thermal: self.thermal + other.thermal,
            electric: self.electric + other.electric,
            gravitational: self.gravitational + other.gravitational,
        }
    }

    /// How much of a threat is absorbed. Returns 0.0–1.0.
    /// If protection >= threat, returns 1.0 (fully absorbed).
    pub fn absorb(protection: u32, threat: u32) -> f32 {
        if threat == 0 {
            return 1.0;
        }
        (protection as f32 / threat as f32).min(1.0)
    }
}
