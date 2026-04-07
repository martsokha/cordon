//! NPC loadout archetypes — data-driven recipes for generating gear.
//!
//! Each faction has one [`ArchetypeDef`] file containing a [`RankLoadout`]
//! per [`Rank`](crate::primitive::Rank). The loadout generator picks the
//! row matching the NPC's faction and rank, then rolls weapons, ammo,
//! armor, and consumables from its weighted pools.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::faction::Faction;
use crate::item::Item;
use crate::primitive::{Id, IdMarker, Rank};

/// Marker for archetype IDs (one per faction).
pub struct Archetype;
impl IdMarker for Archetype {}

/// All loadout recipes for a single faction, one per rank.
///
/// The [`id`](ArchetypeDef::id) doubles as the faction this archetype
/// applies to — `id == faction_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeDef {
    /// Archetype ID. Equal to the faction ID this archetype generates for.
    pub id: Id<Archetype>,
    /// The faction this archetype is for.
    pub faction: Id<Faction>,
    /// One loadout recipe per rank.
    pub ranks: HashMap<Rank, RankLoadout>,
}

impl ArchetypeDef {
    /// Look up the loadout recipe for a rank, falling back to lower ranks
    /// if the requested one is missing.
    pub fn for_rank(&self, rank: Rank) -> Option<&RankLoadout> {
        // Try the requested rank first, then walk down to Novice.
        for r in Rank::all().into_iter().rev() {
            if r <= rank
                && let Some(loadout) = self.ranks.get(&r)
            {
                return Some(loadout);
            }
        }
        None
    }
}

/// A weighted item entry in a pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedItem {
    pub id: Id<Item>,
    /// Selection weight. Higher = more likely.
    #[serde(default = "one")]
    pub weight: u32,
}

fn one() -> u32 {
    1
}

/// The loadout recipe for one (faction, rank) cell.
///
/// All pools are weighted lists of item IDs. The generator rolls each
/// pool and instantiates the chosen def via `ItemInstance::new`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RankLoadout {
    /// Primary weapon options.
    #[serde(default)]
    pub primary: Vec<WeightedItem>,
    /// Secondary (sidearm) options.
    #[serde(default)]
    pub secondary: Vec<WeightedItem>,
    /// Body armor options.
    #[serde(default)]
    pub armor: Vec<WeightedItem>,
    /// Helmet options.
    #[serde(default)]
    pub helmet: Vec<WeightedItem>,
    /// Number of fresh ammo boxes the NPC carries for their primary.
    #[serde(default = "one")]
    pub ammo_boxes: u32,
    /// Number of fresh ammo boxes the NPC carries for their secondary.
    #[serde(default)]
    pub secondary_ammo_boxes: u32,
    /// Consumable options. Generator rolls one of each up to count.
    #[serde(default)]
    pub consumables: Vec<WeightedItem>,
    /// How many consumable items to roll into the general pouch.
    #[serde(default)]
    pub consumable_count: u32,
}
