//! Concrete item instances in the game world.

use serde::{Deserialize, Serialize};

use super::data::ItemData;
use super::def::{Item as ItemMarker, ItemDef};
use crate::primitive::{Condition, Id};

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

/// A single item instance in the game world.
///
/// Items do not merge — each instance is tracked individually with
/// its own durability and authenticity. References an [`ItemDef`]
/// by [`Id`].
///
/// Durability is an integer budget. Each hit absorbed by armor or shot
/// fired by a weapon subtracts from it. At 0, the item is broken.
/// Items whose [`ItemDef::durability`] is `None` are indestructible
/// (consumables, ammo, documents) and carry `durability = None`.
///
/// `count` represents stack size for items that come in multiples:
/// rounds in a box of ammo, rounds chambered in a weapon's magazine,
/// items in a pack. Defaults to 1.
///
/// `loaded_ammo` is only meaningful for weapon instances; it records
/// which ammo def's rounds are currently in the magazine, so the
/// combat system reads accurate damage and penetration values without
/// having to guess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInstance {
    /// ID of the [`ItemDef`] this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
    /// Current durability remaining. `None` for indestructible items.
    pub durability: Option<u32>,
    /// Stack size (rounds in mag/box, items in a pack). Default 1.
    pub count: u32,
    /// For weapons: the ammo def whose rounds are in the magazine.
    /// `None` means the magazine is empty or unloaded.
    #[serde(default)]
    pub loaded_ammo: Option<Id<ItemMarker>>,
}

impl ItemInstance {
    /// Create a fresh, undamaged instance from a definition.
    ///
    /// Durability starts at the def's max (or `None` if indestructible).
    ///
    /// `count` semantics depend on the item type:
    /// - **Ammo**: rounds remaining in this box (starts full).
    /// - **Weapon**: rounds chambered in the magazine (starts at 0 — the
    ///   loadout generator or a reload action fills it).
    /// - Everything else: stack size (default 1).
    pub fn new(def: &ItemDef) -> Self {
        let count = match &def.data {
            ItemData::Ammo(a) => a.quantity,
            _ => 1,
        };
        Self {
            def_id: def.id.clone(),
            authenticity: Authenticity::Genuine,
            durability: def.durability,
            count,
            loaded_ammo: None,
        }
    }

    /// Compute the 0–1 condition view from this instance's durability
    /// and the def's max durability. `None` for indestructible items.
    pub fn condition(&self, def: &ItemDef) -> Option<Condition> {
        match (self.durability, def.durability) {
            (Some(cur), Some(max)) => Some(Condition::from_durability(cur, max)),
            _ => None,
        }
    }

    /// Subtract durability. No-op for indestructible items.
    pub fn degrade(&mut self, amount: u32) {
        if let Some(d) = &mut self.durability {
            *d = d.saturating_sub(amount);
        }
    }

    /// Whether this item has been destroyed (durability hit zero).
    /// Indestructible items always return `false`.
    pub fn is_broken(&self) -> bool {
        matches!(self.durability, Some(0))
    }
}
