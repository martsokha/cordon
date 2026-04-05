use std::collections::HashMap;

use cordon_core::primitive::id::Id;

/// A weighted entry in a loot table.
///
/// Each entry represents a possible drop: an item ID, a weight for
/// random selection, and ranges for condition and quantity.
#[derive(Debug, Clone)]
pub struct LootEntry {
    /// Item definition ID.
    pub item_id: Id,
    /// Selection weight (higher = more likely).
    pub weight: u32,
    /// Minimum condition of dropped item (0.0–1.0).
    pub min_condition: f32,
    /// Maximum condition of dropped item (0.0–1.0).
    pub max_condition: f32,
    /// Minimum quantity per drop.
    pub min_quantity: u32,
    /// Maximum quantity per drop.
    pub max_quantity: u32,
}

/// Loot table for a specific sector.
#[derive(Debug, Clone, Default)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// Sum of all entry weights (for probability calculation).
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|e| e.weight).sum()
    }
}

/// All loot tables, keyed by sector ID.
#[derive(Debug, Clone, Default)]
pub struct LootTables {
    pub tables: HashMap<Id, LootTable>,
}

impl LootTables {
    /// Get the loot table for a sector.
    pub fn get(&self, sector: &Id) -> Option<&LootTable> {
        self.tables.get(sector)
    }
}
