//! Armor item data (body armor and head protection).

use serde::{Deserialize, Serialize};

use crate::primitive::Resistances;

/// Which armor slot this piece of armor occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArmorSlot {
    /// Body armor: jackets, vests, suits.
    Suit,
    /// Head protection: helmets, balaclavas.
    Helmet,
}

/// Data for armor items.
///
/// Armor occupies its own equipment slot (not inventory slots).
/// All protection values are absolute — compared directly against
/// the corresponding threat value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Which slot this armor occupies.
    pub slot: ArmorSlot,
    /// Protection ratings against all damage types.
    pub resistances: Resistances,
    /// Extra inventory slots granted while wearing this armor.
    pub bonus_slots: u8,
    /// How quickly the armor degrades with damage (higher = faster wear).
    pub durability: f32,
}
