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
/// Armor occupies its own equipment slot in a [`Loadout`](crate::item::Loadout).
/// All protection values are absolute — compared directly against the
/// corresponding threat value via [`Resistances::resolve_hit`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorData {
    /// Which slot this armor occupies.
    pub slot: ArmorSlot,
    /// Protection ratings against all damage types.
    pub resistances: Resistances,
    /// Extra general inventory slots granted while wearing this armor
    /// (cargo pouches, harness loops).
    pub inventory_slots: u8,
    /// Number of relic slots this armor exposes. Capped at 4. Helmets
    /// always have 0; suits vary by design.
    #[serde(default)]
    pub relic_slots: u8,
}
