//! Weapon item data (firearms).

use serde::{Deserialize, Serialize};

use super::Caliber;
use crate::primitive::distance::Distance;
use crate::primitive::id::Id;

/// Weapon fire mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FireMode {
    /// One shot per trigger pull.
    Semi,
    /// Fixed-length burst per trigger pull (e.g., 2-round for AN-94).
    Burst(u8),
    /// Continuous fire while trigger held.
    Auto,
}

/// Data for weapon items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponData {
    /// Caliber ID this weapon fires. Must match an ammo item's caliber.
    pub caliber: Id<Caliber>,
    /// Available fire modes (e.g., `[Semi, Auto]` for an AK-74).
    pub fire_modes: Vec<FireMode>,
    /// Rounds per second at full auto/burst.
    pub fire_rate: f32,
    /// Base accuracy (0.0–1.0). Higher = tighter spread.
    pub accuracy: f32,
    /// Recoil per shot (0.0–1.0). Higher = more spread over sustained fire.
    pub recoil: f32,
    /// Magazine capacity in rounds.
    pub magazine: u32,
    /// Effective range in meters.
    pub effective_range: Distance,
    /// Whether this weapon is suppressed (affects runner stealth missions).
    pub suppressed: bool,
}
