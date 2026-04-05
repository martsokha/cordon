use std::collections::HashMap;

use cordon_core::primitive::id::{Area, Id, Item};
use serde::{Deserialize, Serialize};

/// A weighted entry in a loot table.
///
/// Each entry represents a possible drop: an item ID, a weight for
/// random selection, and ranges for condition and quantity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    /// Item definition ID.
    pub item_id: Id<Item>,
    /// Selection weight (higher = more likely).
    pub weight: u32,
    /// Minimum condition of dropped item (0.0–1.0).
    pub min_condition: f32,
    /// Maximum condition of dropped item (0.0–1.0).
    pub max_condition: f32,
}

/// Loot table for a specific area.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// Sum of all entry weights (for probability calculation).
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|e| e.weight).sum()
    }
}

/// All loot tables, keyed by area ID.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LootTables {
    pub tables: HashMap<Id<Area>, LootTable>,
}

impl LootTables {
    /// Get the loot table for an area.
    pub fn get(&self, area: &Id<Area>) -> Option<&LootTable> {
        self.tables.get(area)
    }
}
