//! Weapon item data (firearms).

use serde::{Deserialize, Serialize};

use super::Caliber;
use crate::primitive::{Distance, Id};

/// Data for weapon items.
///
/// Weapons do not track magazine capacity or chambered rounds —
/// the sim consumes one round per shot directly from a matching
/// ammo box in the general pouch, with no reload step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponData {
    /// Caliber ID this weapon fires. Must match an ammo item's caliber.
    pub caliber: Id<Caliber>,
    /// Rounds per second at full auto/burst.
    pub fire_rate: f32,
    /// Base accuracy (0.0–1.0). Higher = tighter spread.
    pub accuracy: f32,
    /// Effective firing range in map units.
    pub range: Distance,
    /// Whether this weapon is suppressed (affects runner stealth missions).
    pub suppressed: bool,
    /// Bonus damage on top of the ammo's base damage (long barrel,
    /// custom load, hand-tuned action). Defaults to 0.
    #[serde(default)]
    pub added_damage: u32,
}
