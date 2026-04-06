//! Slot-based inventory for NPCs and the bunker.

use serde::{Deserialize, Serialize};

use super::instance::Item;

/// A slot-based inventory. Each item occupies one slot.
///
/// Used for NPC gear, bunker storage, and hidden storage.
/// The capacity is fixed at creation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Maximum number of items this inventory can hold.
    capacity: u8,
    /// Items currently stored.
    items: Vec<Item>,
}

impl Inventory {
    /// Create a new empty inventory with the given capacity.
    pub fn new(capacity: u8) -> Self {
        Self {
            capacity,
            items: Vec::new(),
        }
    }

    /// Maximum number of items this inventory can hold.
    pub fn capacity(&self) -> u8 {
        self.capacity
    }

    /// Number of items currently stored.
    pub fn len(&self) -> u8 {
        self.items.len() as u8
    }

    /// Whether the inventory is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether the inventory is full.
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Number of free slots.
    pub fn free_slots(&self) -> u8 {
        self.capacity.saturating_sub(self.len())
    }

    /// Try to add an item. Returns `Err(item)` if full.
    pub fn add(&mut self, item: Item) -> Result<(), Item> {
        if self.is_full() {
            Err(item)
        } else {
            self.items.push(item);
            Ok(())
        }
    }

    /// Remove an item by index. Returns `None` if out of bounds.
    pub fn remove(&mut self, index: usize) -> Option<Item> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Get a reference to all items.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Get a mutable reference to all items.
    pub fn items_mut(&mut self) -> &mut [Item] {
        &mut self.items
    }

    /// Set a new capacity. Does not remove items if the new capacity
    /// is smaller than the current item count.
    pub fn set_capacity(&mut self, capacity: u8) {
        self.capacity = capacity;
    }
}
