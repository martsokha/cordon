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
/// `count` represents stack size for items that come in multiples
/// (a box of ammo, a pack of bandages). Defaults to 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInstance {
    /// ID of the [`ItemDef`] this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
    /// Current durability remaining. `None` for indestructible items.
    pub durability: Option<u32>,
    /// Stack size (rounds in a box of ammo, items in a pack). Default 1.
    pub count: u32,
}

impl ItemInstance {
    /// Create a fresh, undamaged instance from a definition.
    ///
    /// Durability starts at the def's max (or `None` if indestructible).
    /// Ammo instances start with a full box (`count = ammo.quantity`);
    /// other items default to `count = 1`.
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
