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
    /// Caliber ID this ammo belongs to (e.g., `"9x18mm"`).
    pub caliber: Id<Caliber>,
    /// Base damage per round.
    pub damage: f32,
    /// Armor penetration value (0.0–1.0). Higher = better against armor.
    pub penetration: f32,
    /// Number of rounds per box.
    pub quantity: u32,
}
