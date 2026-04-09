use std::collections::HashMap;

use cordon_core::entity::archetype::{Archetype, ArchetypeDef};
use cordon_core::entity::bunker::{Upgrade, UpgradeDef};
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::entity::name::{NamePool, NamePoolMarker};
use cordon_core::entity::perk::{Perk, PerkDef};
use cordon_core::item::{Item, ItemDef};
use cordon_core::primitive::Id;
use cordon_core::world::area::{Area, AreaDef};
use cordon_core::world::loot::LootTables;
use cordon_core::world::narrative::{
    Event, EventDef, Quest, QuestDef, QuestTrigger, QuestTriggerDef,
};

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
/// All lookups are by typed ID aliases from [`cordon_core::primitive`].
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
    /// Quest trigger rules keyed by trigger ID. Each trigger
    /// references the quest it starts via [`QuestTriggerDef::quest`].
    pub triggers: HashMap<Id<QuestTrigger>, QuestTriggerDef>,
    /// Name pools keyed by pool ID.
    pub name_pools: HashMap<Id<NamePoolMarker>, NamePool>,
    /// Loot tables keyed by area ID.
    pub loot_tables: LootTables,
    /// NPC loadout archetypes keyed by faction ID (one per faction).
    pub archetypes: HashMap<Id<Archetype>, ArchetypeDef>,
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

    /// Look up the loadout archetype for a faction.
    ///
    /// Walks `archetypes.values()` filtering on the
    /// [`ArchetypeDef::faction`] field rather than key-lookup by
    /// string. The HashMap key is an archetype ID
    /// (`archetype_garrison`), not a faction ID
    /// (`faction_garrison`), so a direct `.get` won't work — and
    /// even if we normalized the key, the `faction` field is the
    /// real cross-reference. Cost is O(N) where N ≤ 5.
    pub fn archetype_for_faction(&self, faction: &Id<Faction>) -> Option<&ArchetypeDef> {
        self.archetypes.values().find(|a| &a.faction == faction)
    }

    /// Get all faction IDs.
    pub fn faction_ids(&self) -> Vec<Id<Faction>> {
        self.factions.keys().cloned().collect()
    }

    /// Get all area IDs.
    pub fn area_ids(&self) -> Vec<Id<Area>> {
        self.areas.keys().cloned().collect()
    }

    /// Build a faction-to-namepool mapping for NPC generation.
    ///
    /// Resolves each faction's `name_pool` ID to the actual [`NamePool`].
    /// Returns a map keyed by faction ID for use with the simulation layer.
    pub fn faction_name_pools(&self) -> HashMap<Id<Faction>, NamePool> {
        self.factions
            .iter()
            .filter_map(|(fid, fdef)| {
                self.name_pools
                    .get(&fdef.namepool)
                    .map(|pool| (fid.clone(), pool.clone()))
            })
            .collect()
    }
}
