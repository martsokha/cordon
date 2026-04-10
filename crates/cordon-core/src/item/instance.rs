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
/// `count` represents stack size for items that come in multiples:
/// rounds in a box of ammo, rounds chambered in a weapon's magazine,
/// items in a pack. Defaults to 1.
///
/// `loaded_ammo` is only meaningful for weapon instances; it records
/// which ammo def's rounds are currently in the magazine, so the
/// combat system reads accurate damage and penetration values without
/// having to guess.
#[derive(Debug, Clone)]
#[derive(Component, Serialize, Deserialize)]
pub struct ItemInstance {
    /// ID of the [`ItemDef`] this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Stack size (rounds in mag/box, items in a pack). Default 1.
    pub count: u32,
    /// For weapons: the ammo def whose rounds are in the magazine.
    /// `None` means the magazine is empty or unloaded.
    #[serde(default)]
    pub loaded_ammo: Option<Id<ItemMarker>>,
}

impl ItemInstance {
    /// Create a fresh instance from a definition.
    ///
    /// `count` semantics depend on the item type:
    /// - **Ammo**: rounds remaining in this box (starts full).
    /// - **Weapon**: rounds chambered in the magazine (starts at 0 — the
    ///   loadout generator fills it, combat tops it up from pouches).
    /// - Everything else: stack size (default 1).
    pub fn new(def: &ItemDef) -> Self {
        let count = match &def.data {
            ItemData::Ammo(a) => a.quantity,
            _ => 1,
        };
        Self {
            def_id: def.id.clone(),
            count,
            loaded_ammo: None,
        }
    }
}
