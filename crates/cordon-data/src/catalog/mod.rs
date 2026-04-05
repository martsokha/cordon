use std::collections::HashMap;

use cordon_core::bunker::UpgradeDef;
use cordon_core::entity::faction::FactionDef;
use cordon_core::entity::npc::PerkDef;
use cordon_core::item::ItemDef;
use cordon_core::primitive::id::Id;
use cordon_core::world::sector::SectorDef;

use crate::loot::LootTables;

/// The read-only game database.
///
/// Loaded once at startup from JSON config files. Contains all static
/// definitions that the simulation references: items, factions, sectors,
/// perks, upgrades, and loot tables.
///
/// Calibers are implicit — they exist because ammo and weapon items
/// reference the same caliber ID string. No separate caliber registry.
/// Player ranks are hardcoded in [`PlayerRank`](cordon_core::entity::player::PlayerRank).
///
/// All lookups are by [`Id`] — the string-based identifier used across
/// all config files.
pub struct GameData {
    /// Item definitions keyed by item ID.
    pub items: HashMap<Id, ItemDef>,
    /// Faction definitions keyed by faction ID.
    pub factions: HashMap<Id, FactionDef>,
    /// Sector definitions keyed by sector ID.
    pub sectors: HashMap<Id, SectorDef>,
    /// Perk definitions keyed by perk ID.
    pub perks: HashMap<Id, PerkDef>,
    /// Upgrade definitions keyed by upgrade ID.
    pub upgrades: HashMap<Id, UpgradeDef>,
    /// Loot tables keyed by sector ID.
    pub loot_tables: LootTables,
}

impl GameData {
    /// Look up an item definition by ID.
    pub fn item(&self, id: &Id) -> Option<&ItemDef> {
        self.items.get(id)
    }

    /// Look up a faction definition by ID.
    pub fn faction(&self, id: &Id) -> Option<&FactionDef> {
        self.factions.get(id)
    }

    /// Look up a sector definition by ID.
    pub fn sector(&self, id: &Id) -> Option<&SectorDef> {
        self.sectors.get(id)
    }

    /// Look up a perk definition by ID.
    pub fn perk(&self, id: &Id) -> Option<&PerkDef> {
        self.perks.get(id)
    }

    /// Get all faction IDs.
    pub fn faction_ids(&self) -> Vec<Id> {
        self.factions.keys().cloned().collect()
    }

    /// Get all sector IDs.
    pub fn sector_ids(&self) -> Vec<Id> {
        self.sectors.keys().cloned().collect()
    }
}
