//! Item definitions loaded from config.

use serde::{Deserialize, Serialize};

use super::category::ItemCategory;
use super::data::ItemData;
use crate::primitive::id::Id;
use crate::primitive::rarity::Rarity;

/// Static item definition loaded from config.
///
/// This is the immutable template for an item type. Concrete instances
/// in the game world are represented by [`Item`](super::Item).
///
/// The [`id`](ItemDef::id) field doubles as the localization key and
/// asset ID — display names are resolved from localization files, and
/// icons are resolved via convention (e.g., `icons/{id}.png`).
/// Calibers are implicit: they exist because ammo and weapon items
/// reference the same caliber ID string. No separate caliber config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique identifier, localization key, and asset ID
    /// (e.g., `"ak74"`, `"medkit"`, `"9x18mm"`).
    pub id: Id,
    /// Type-specific data: consumable effects, weapon caliber, etc.
    pub data: ItemData,
    /// Base price at condition 1.0 with no market modifiers.
    pub base_price: u32,
    /// Faction IDs of factions that supply this item.
    pub suppliers: Vec<Id>,
    /// How rare this item is. Affects loot tables and NPC behavior.
    pub rarity: Rarity,
    /// How many inventory slots this item occupies when carried.
    pub slots: u8,
    /// How fast this item's condition degrades with use (1.0 = normal rate).
    /// Lower = more durable. Applies to weapons, armor, and anything that wears.
    pub durability: f32,
}

impl ItemDef {
    /// Get the simple [`ItemCategory`] for this item.
    pub fn category(&self) -> ItemCategory {
        self.data.category()
    }
}
