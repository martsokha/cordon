//! Ammunition item data.

use serde::{Deserialize, Serialize};

use super::Caliber;
use crate::primitive::Id;

/// Data for ammunition items.
///
/// Comes in boxes — [`quantity`](AmmoData::quantity) is rounds per box
/// as purchased. References a caliber by ID; weapons that fire the
/// same caliber ID can use this ammo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmoData {
    /// Caliber ID this ammo belongs to (e.g., `"9x19mm"`).
    pub caliber: Id<Caliber>,
    /// Base damage per round in HP.
    pub damage: u32,
    /// Armor penetration value. Compared against armor's protection
    /// value via [`Resistances::resolve_hit`](crate::primitive::Resistances::resolve_hit).
    /// Typical range: 5 (pistol) to 40 (AP rifle).
    pub penetration: u32,
    /// Number of rounds per fresh box.
    pub quantity: u32,
}
