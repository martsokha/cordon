//! Item and caliber definitions loaded from config.

use serde::{Deserialize, Serialize};

use crate::primitive::id::Id;
use super::category::ItemCategory;
use super::data::ItemData;

/// Static item definition loaded from config.
///
/// This is the immutable template for an item type. Concrete instances
/// in the game world are represented by [`Item`](super::Item).
///
/// The [`id`](ItemDef::id) field doubles as the localization key —
/// display names are resolved at render time from language-specific
/// localization files, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique identifier and localization key
    /// (e.g., `"ak74"`, `"medkit"`, `"9x18mm"`).
    pub id: Id,
    /// Icon asset path or sprite ID for rendering.
    pub icon: String,
    /// Type-specific data: consumable effects, weapon caliber, etc.
    pub data: ItemData,
    /// Base price at condition 1.0 with no market modifiers.
    pub base_price: u32,
    /// Faction IDs of factions that supply this item.
    pub suppliers: Vec<Id>,
    /// Rarity tier (e.g., `"common"`, `"rare"`, `"legendary"`).
    pub rarity: Option<String>,
    /// How many inventory slots this item occupies.
    pub slots: u32,
}

impl ItemDef {
    /// Get the simple [`ItemCategory`] for this item.
    pub fn category(&self) -> ItemCategory {
        self.data.category()
    }
}

/// Caliber definition loaded from config.
///
/// Defines an ammunition caliber and links ammo items to the weapons
/// that fire them. The [`id`](CaliberDef::id) doubles as the
/// localization key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaliberDef {
    /// Unique identifier and localization key (e.g., `"9x18mm"`, `"12ga"`).
    pub id: Id,
}
