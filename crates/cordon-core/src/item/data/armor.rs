//! Armor item data (body armor and head protection).

use serde::{Deserialize, Serialize};

/// Which armor slot this piece of armor occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    /// Body armor: jackets, vests, suits.
    Suit,
    /// Head protection: helmets, balaclavas.
    Helmet,
}

/// Data for armor items.
///
/// Armor occupies its own equipment slot (not inventory slots) and
/// can grant bonus inventory slots while equipped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Which slot this armor occupies.
    pub slot: ArmorSlot,
    /// Ballistic protection (0.0–1.0). Fraction of bullet damage absorbed.
    pub ballistic_protection: f32,
    /// Radiation protection (0.0–1.0). Fraction of radiation absorbed.
    pub radiation_protection: f32,
    /// Extra inventory slots granted while wearing this armor.
    pub bonus_slots: u8,
}
