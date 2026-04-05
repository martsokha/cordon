use std::collections::HashMap;

use cordon_core::bunker::UpgradeDef;
use cordon_core::item::{CaliberDef, ItemDef};
use cordon_core::entity::faction::FactionDef;
use cordon_core::entity::npc::PerkDef;
use cordon_core::entity::player::PlayerRankDef;
use cordon_core::primitive::id::Id;
use cordon_core::world::sector::SectorDef;

use crate::loot::LootTables;

/// The read-only game database.
///
/// Loaded once at startup from JSON config files. Contains all static
/// definitions that the simulation references: items, factions, sectors,
/// calibers, perks, upgrades, player ranks, and loot tables.
///
/// All lookups are by [`Id`] — the string-based identifier used across
/// all config files.
pub struct GameData {
    /// Item definitions keyed by item ID.
    pub items: HashMap<Id, ItemDef>,
    /// Caliber definitions keyed by caliber ID.
    pub calibers: HashMap<Id, CaliberDef>,
    /// Faction definitions keyed by faction ID.
    pub factions: HashMap<Id, FactionDef>,
    /// Sector definitions keyed by sector ID.
    pub sectors: HashMap<Id, SectorDef>,
    /// Perk definitions keyed by perk ID.
    pub perks: HashMap<Id, PerkDef>,
    /// Upgrade definitions keyed by upgrade ID.
    pub upgrades: HashMap<Id, UpgradeDef>,
    /// Player rank definitions, ordered by tier.
    pub player_ranks: Vec<PlayerRankDef>,
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

    /// Look up a caliber definition by ID.
    pub fn caliber(&self, id: &Id) -> Option<&CaliberDef> {
        self.calibers.get(id)
    }

    /// Get all faction IDs.
    pub fn faction_ids(&self) -> Vec<Id> {
        self.factions.keys().cloned().collect()
    }

    /// Get all sector IDs.
    pub fn sector_ids(&self) -> Vec<Id> {
        self.sectors.keys().cloned().collect()
    }

    /// Get the maximum squad count for a given player rank tier.
    pub fn max_squads_for_rank(&self, tier: u8) -> u8 {
        self.player_ranks
            .iter()
            .find(|r| r.tier == tier)
            .map(|r| r.max_squads)
            .unwrap_or(2)
    }
}
