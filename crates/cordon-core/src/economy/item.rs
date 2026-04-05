//! Item definitions, calibers, and item instances.
//!
//! [`ItemDef`] and [`CaliberDef`] are loaded from config files.
//! [`ItemStack`] represents concrete item instances in the game world.

use serde::{Deserialize, Serialize};

use crate::object::id::Id;

/// Broad category of an item. Determines how it's displayed, stored, and traded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    /// Consumable food and drink. May spoil.
    Food,
    /// Medical supplies: bandages, medkits, pills.
    Med,
    /// Ammunition, sold in boxes.
    Ammo,
    /// Firearms: pistols, rifles, shotguns, etc.
    Weapon,
    /// Head protection: helmets, balaclavas.
    Helmet,
    /// Body protection: jackets, vests, suits.
    Suit,
    /// Zone relics with anomalous properties.
    Relic,
    /// Intel: PDAs, reports, patrol routes.
    Document,
    /// Experimental equipment from the Institute.
    Tech,
    /// Hand grenades and launcher ammo.
    Grenade,
    /// Weapon attachments: underbarrel launchers, etc.
    Attachment,
}

/// Which armor slot an item occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    /// Body armor slot.
    Suit,
    /// Helmet slot.
    Helmet,
}

/// Stability state of a relic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicStability {
    /// Contained properly, safe to store and sell.
    Stable,
    /// Not contained, degrades over time, may harm handler.
    Unstable,
    /// Depleted or damaged, minimal value.
    Inert,
}

/// Whether an item is genuine or has been tampered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Authenticity {
    /// Real, unmodified item.
    Genuine,
    /// Fake: looks real but has no properties (or harmful ones).
    Counterfeit,
    /// Past its effective date (meds, food).
    Expired,
    /// Planted by a faction to mislead (documents).
    Doctored,
}

/// Static item definition loaded from config.
///
/// This is the immutable template for an item type. Concrete instances
/// in the game world are represented by [`ItemStack`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique identifier for this item type.
    pub id: Id,
    /// Display name.
    pub name: String,
    /// Broad category.
    pub kind: ItemKind,
    /// Base price at condition 1.0 with no modifiers.
    pub base_price: u32,
    /// Caliber ID. For ammo: what caliber this is. For weapons: what caliber it fires.
    pub caliber: Option<Id>,
    /// Faction IDs of factions that supply this item.
    pub suppliers: Vec<Id>,
    /// Days until spoilage. `None` means the item never spoils.
    pub spoil_days: Option<u32>,
    /// Rarity label (e.g., "common", "rare", "legendary").
    pub rarity: Option<String>,
    /// Which armor slot this item occupies, if it's armor.
    pub armor_slot: Option<ArmorSlot>,
}

/// Caliber definition loaded from config.
///
/// Links ammo items to the weapons that fire them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaliberDef {
    /// Unique identifier (e.g., `"9x18mm"`, `"5.45x39mm"`).
    pub id: Id,
    /// Display name (e.g., `"9x18mm PM"`).
    pub name: String,
    /// Short description of this caliber's characteristics.
    pub description: String,
}

/// A concrete item instance in the game world.
///
/// Represents a stack of items with a specific condition, authenticity,
/// and freshness state. References an [`ItemDef`] by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    /// ID of the [`ItemDef`] this stack is an instance of.
    pub def_id: Id,
    /// Number of items in this stack.
    pub quantity: u32,
    /// Condition from 0.0 (destroyed) to 1.0 (factory new).
    /// Price scales with condition² (see [`PriceModifiers::final_price`]).
    pub condition: f32,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
    /// Relic stability state, if this item is a relic.
    pub relic_stability: Option<RelicStability>,
    /// Days remaining until spoilage. `None` means it doesn't spoil.
    pub freshness: Option<u32>,
}

impl ItemStack {
    /// Create a new genuine item stack with the given condition.
    pub fn new(def_id: Id, quantity: u32, condition: f32) -> Self {
        Self {
            def_id,
            quantity,
            condition: condition.clamp(0.0, 1.0),
            authenticity: Authenticity::Genuine,
            relic_stability: None,
            freshness: None,
        }
    }
}
