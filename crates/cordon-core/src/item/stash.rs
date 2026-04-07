//! Slot-based bulk storage for the bunker and hidden caches.

use serde::{Deserialize, Serialize};

use super::instance::ItemInstance;

/// A slot-based storage container with a fixed capacity.
///
/// Used for bunker storage and hidden caches — bulk holding without
/// any equipment-slot semantics. NPCs use [`Loadout`](super::Loadout)
/// instead, which has typed equipment slots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stash {
    /// Maximum number of items this stash can hold.
    capacity: u8,
    /// Items currently stored.
    items: Vec<ItemInstance>,
}

impl Stash {
    /// Create a new empty stash with the given capacity.
    pub fn new(capacity: u8) -> Self {
        Self {
            capacity,
            items: Vec::new(),
        }
    }

    /// Maximum number of items this stash can hold.
    pub fn capacity(&self) -> u8 {
        self.capacity
    }

    /// Number of items currently stored.
    pub fn len(&self) -> u8 {
        self.items.len() as u8
    }

    /// Whether the stash is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether the stash is full.
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Number of free slots.
    pub fn free_slots(&self) -> u8 {
        self.capacity.saturating_sub(self.len())
    }

    /// Try to add an item. Returns `Err(item)` if full.
    pub fn add(&mut self, item: ItemInstance) -> Result<(), ItemInstance> {
        if self.is_full() {
            Err(item)
        } else {
            self.items.push(item);
            Ok(())
        }
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

    /// Set a new capacity. Does not remove items if shrinking.
    pub fn set_capacity(&mut self, capacity: u8) {
        self.capacity = capacity;
    }
}
