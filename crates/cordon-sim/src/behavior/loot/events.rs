//! Loot events.

use bevy::prelude::*;
use cordon_core::item::Item;
use cordon_core::primitive::Id;

/// A looter pulled an item from a corpse into their general
/// pouch.
#[derive(Message, Debug, Clone)]
pub struct ItemLooted {
    pub looter: Entity,
    pub corpse: Entity,
    pub item: Id<Item>,
}
