//! Concrete item instances in the game world.

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use super::data::ItemData;
use super::def::{Item as ItemMarker, ItemDef};
use crate::primitive::Id;

/// A single item instance in the game world.
///
/// Items do not merge — each instance is tracked individually and
/// references an [`ItemDef`] by [`Id`].
///
/// `count` represents stack size for stackable items: rounds in a
/// box of ammo, items in a pack. Non-stackable items (weapons,
/// armor, consumables) default to 1.
///
/// Weapons deliberately do not carry a "rounds in magazine"
/// count — the sim consumes one round per shot directly from a
/// matching ammo box in the general pouch. There is no reload step.
#[derive(Debug, Clone)]
#[derive(Component, Serialize, Deserialize)]
pub struct ItemInstance {
    /// ID of the [`ItemDef`] this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Stack size (rounds in a box, items in a pack). Default 1.
    pub count: u32,
}

impl ItemInstance {
    /// Create a fresh instance from a definition.
    ///
    /// `count` semantics depend on the item type:
    /// - **Ammo**: rounds remaining in this box (starts full from
    ///   the def's `quantity`).
    /// - Everything else: stack size 1.
    pub fn new(def: &ItemDef) -> Self {
        let count = match &def.data {
            ItemData::Ammo(a) => a.quantity,
            _ => 1,
        };
        Self {
            def_id: def.id.clone(),
            count,
        }
    }
}
