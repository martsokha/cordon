//! Item definitions loaded from config.

use serde::{Deserialize, Serialize};

use super::category::ItemCategory;
use super::data::ItemData;
use crate::entity::faction::Faction;
use crate::primitive::{Credits, Id, IdMarker, Rarity};

/// Marker for item definition IDs.
pub struct Item;
impl IdMarker for Item {}

/// A faction that supplies this item, with a price multiplier.
///
/// Different factions sell the same item at different prices.
/// A multiplier of 1.0 means base price, 0.8 means 20% cheaper,
/// 1.5 means 50% more expensive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supplier {
    /// Faction ID of the supplier.
    pub faction: Id<Faction>,
    /// Price multiplier for this supplier (1.0 = base price).
    pub price_multiplier: f32,
}

/// Static item definition loaded from config.
///
/// This is the immutable template for an item type. Concrete instances
/// in the game world are represented by [`Item`](super::Item).
///
/// All items occupy exactly one inventory slot.
///
/// The [`id`](ItemDef::id) field doubles as the localization key and
/// asset ID — display names are resolved from localization files, and
/// icons are resolved via convention (e.g., `icons/{id}.png`).
/// Calibers are implicit: they exist because ammo and weapon items
/// reference the same caliber ID string. No separate caliber config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique identifier, localization key, and asset ID
    /// (e.g., `"m4_carbine"`, `"medkit"`, `"9x19mm"`).
    pub id: Id<Item>,
    /// Type-specific data: consumable effects, weapon caliber, etc.
    pub data: ItemData,
    /// Base price with no market modifiers.
    pub base_price: Credits,
    /// Factions that supply this item, each with a price multiplier.
    pub suppliers: Vec<Supplier>,
    /// How rare this item is. Affects loot tables and NPC behavior.
    pub rarity: Rarity,
}

impl ItemDef {
    /// Get the simple [`ItemCategory`] for this item.
    pub fn category(&self) -> ItemCategory {
        self.data.category()
    }
}
