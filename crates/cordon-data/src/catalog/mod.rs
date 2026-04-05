use std::collections::HashMap;

use cordon_core::bunker::UpgradeDef;
use cordon_core::entity::faction::FactionDef;
use cordon_core::entity::name::{NamePool, NamePoolMarker};
use cordon_core::entity::npc::PerkDef;
use cordon_core::item::ItemDef;
use cordon_core::primitive::id::{Area, Event, Faction, Id, Item, Perk, Quest, Upgrade};
use cordon_core::world::area::AreaDef;
use cordon_core::world::event::EventDef;
use cordon_core::world::loot::LootTables;
use cordon_core::world::quest::QuestDef;

/// The read-only game database.
///
/// Loaded once at startup from JSON config files. Contains all static
/// definitions that the simulation references: items, factions, areas,
/// perks, upgrades, and loot tables.
///
/// Calibers are implicit — they exist because ammo and weapon items
/// reference the same caliber ID string. No separate caliber registry.
/// Player ranks are hardcoded in [`PlayerRank`](cordon_core::entity::player::PlayerRank).
///
/// All lookups are by typed ID aliases from [`cordon_core::primitive::id`].
pub struct GameData {
    /// Item definitions keyed by item ID.
    pub items: HashMap<Id<Item>, ItemDef>,
    /// Faction definitions keyed by faction ID.
    pub factions: HashMap<Id<Faction>, FactionDef>,
    /// Area definitions keyed by area ID.
    pub areas: HashMap<Id<Area>, AreaDef>,
    /// Perk definitions keyed by perk ID.
    pub perks: HashMap<Id<Perk>, PerkDef>,
    /// Upgrade definitions keyed by upgrade ID.
    pub upgrades: HashMap<Id<Upgrade>, UpgradeDef>,
    /// Event definitions keyed by event ID.
    pub events: HashMap<Id<Event>, EventDef>,
    /// Quest definitions keyed by quest ID.
    pub quests: HashMap<Id<Quest>, QuestDef>,
    /// Name pools keyed by pool ID.
    pub name_pools: HashMap<Id<NamePoolMarker>, NamePool>,
    /// Loot tables keyed by area ID.
    pub loot_tables: LootTables,
}

impl GameData {
    /// Look up an item definition by ID.
    pub fn item(&self, id: &Id<Item>) -> Option<&ItemDef> {
        self.items.get(id)
    }

    /// Look up a faction definition by ID.
    pub fn faction(&self, id: &Id<Faction>) -> Option<&FactionDef> {
        self.factions.get(id)
    }

    /// Look up an area definition by ID.
    pub fn area(&self, id: &Id<Area>) -> Option<&AreaDef> {
        self.areas.get(id)
    }

    /// Look up a perk definition by ID.
    pub fn perk(&self, id: &Id<Perk>) -> Option<&PerkDef> {
        self.perks.get(id)
    }

    /// Get all faction IDs.
    pub fn faction_ids(&self) -> Vec<Id<Faction>> {
        self.factions.keys().cloned().collect()
    }

    /// Get all area IDs.
    pub fn area_ids(&self) -> Vec<Id<Area>> {
        self.areas.keys().cloned().collect()
    }
}
