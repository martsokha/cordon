//! Unbounded bulk storage for the bunker and hidden caches.
//!
//! Previously enforced a fixed capacity; capacity now comes from
//! physical rack props in the bunker (see `UpgradeEffect::HallRackPair`
//! and eventual rack-as-entity storage). `Stash` is an unbounded
//! container until that rework lands.

use serde::{Deserialize, Serialize};

use super::instance::ItemInstance;

/// A bulk item container. NPCs use [`Loadout`](super::Loadout)
/// instead, which has typed equipment slots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stash {
    items: Vec<ItemInstance>,
}

impl Stash {
    /// Create a new empty stash.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of items currently stored.
    pub fn len(&self) -> u8 {
        self.items.len() as u8
    }

    /// Whether the stash is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add an item. Always succeeds (no capacity).
    pub fn add(&mut self, item: ItemInstance) {
        self.items.push(item);
    }

    /// Remove an item by index. Returns `None` if out of bounds.
    pub fn remove(&mut self, index: usize) -> Option<ItemInstance> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Get a reference to all items.
    pub fn items(&self) -> &[ItemInstance] {
        &self.items
    }

    /// Get a mutable reference to all items.
    pub fn items_mut(&mut self) -> &mut [ItemInstance] {
        &mut self.items
    }
}
