use std::collections::HashMap;

use cordon_core::item::ItemId;
use cordon_core::sector::SectorId;

/// A weighted entry in a loot table.
#[derive(Debug, Clone)]
pub struct LootEntry {
    pub item_id: ItemId,
    pub weight: u32,
    pub min_condition: f32,
    pub max_condition: f32,
    pub min_quantity: u32,
    pub max_quantity: u32,
}

/// Loot table for a specific sector.
#[derive(Debug, Clone, Default)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    pub fn total_weight(&self) -> u32 {
        self.entries.iter().map(|e| e.weight).sum()
    }
}

/// All loot tables, keyed by sector.
#[derive(Debug, Clone, Default)]
pub struct LootTables {
    pub tables: HashMap<SectorId, LootTable>,
}

impl LootTables {
    pub fn get(&self, sector: SectorId) -> Option<&LootTable> {
        self.tables.get(&sector)
    }
}
