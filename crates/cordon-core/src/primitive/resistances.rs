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

    /// Resolve a hit against this much protection, returning the
    /// HP dealt to the target.
    ///
    /// The model is multiplicative: a round whose penetration matches
    /// the protection deals roughly full damage; a round whose
    /// penetration is half the protection deals roughly half. Any hit
    /// always chips at least 1 HP, regardless of how outclassed the
    /// round is.
    pub fn resolve_hit(protection: u32, penetration: u32, damage: u32) -> u32 {
        if damage == 0 {
            return 0;
        }
        let ratio = if protection == 0 {
            1.0
        } else {
            (penetration as f32 / protection as f32).min(1.0)
        };
        let dealt = ((damage as f32) * ratio).round() as u32;
        dealt.max(1).min(damage)
    }
}
