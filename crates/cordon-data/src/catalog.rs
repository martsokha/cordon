use cordon_core::faction::FactionRelation;
use cordon_core::item::ItemDef;
use cordon_core::sector::SectorDef;

use crate::loot::LootTables;

/// The read-only game database. Loaded once, referenced everywhere.
pub struct GameData {
    pub items: Vec<ItemDef>,
    pub sectors: Vec<SectorDef>,
    pub faction_relations: Vec<FactionRelation>,
    pub loot_tables: LootTables,
}

impl GameData {
    pub fn item_by_id(&self, id: cordon_core::item::ItemId) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_by_name(&self, name: &str) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.name == name)
    }
}
