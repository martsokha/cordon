//! NPC type markers + the hidden-personality enum.
//!
//! There's no `Npc` data struct anymore — NPCs are Bevy entities
//! assembled from cordon-core component types (`NpcName`,
//! `Loadout`, `Experience`, `Credits`, `Trust`, `Loyalty`,
//! `Personality`) plus cordon-sim glue (`NpcMarker`,
//! `FactionId`, `NpcAttributes`, `Perks`, `Employment`,
//! `NpcBundle`). The generator in cordon-sim produces bundles
//! directly.
//!
//! What remains here is:
//!
//! - `Npc` — a phantom marker type used as the type parameter
//!   on [`Uid<Npc>`], so stable save-game IDs stay typed.
//! - `Role` / `Personality` — enum flavour types stored as
//!   fields inside components, not as components themselves.
//! - `NpcTemplate` — marker for NPC template IDs used in quest
//!   consequences.

use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::primitive::{Id, IdMarker, Rank, Trust};

use super::faction::Faction;
use super::perk::Perk;

/// Phantom marker for NPC-stable save-game IDs. Used as the
/// type parameter on `Uid<Npc>`. Has no fields — all the actual
/// NPC data lives on the Bevy entity as components.
pub struct Npc;

/// Marker for NPC template IDs (used in quest consequences).
pub struct NpcTemplate;
impl IdMarker for NpcTemplate {}

/// A named, unique NPC definition. Loaded from `assets/data/npcs/`.
///
/// Templates are for story-relevant characters with persistent
/// identities — "Lieutenant Petrov," not "a random Garrison soldier."
/// Generic NPCs are spawned from [`ArchetypeDef`](super::archetype::ArchetypeDef)
/// instead.
///
/// At spawn time the template's rank determines base stats, and gear
/// is either pulled from the faction's archetype pool (if `loadout`
/// is `None`) or set to the authored item list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplateDef {
    pub id: Id<NpcTemplate>,
    /// Localization key for the display name, resolved at render time.
    pub name_key: String,
    pub faction: Id<Faction>,
    /// Base rank — actual spawn XP is randomized within this tier.
    pub rank: Rank,
    pub personality: Personality,
    /// Starting trust toward the player.
    pub trust: Trust,
    pub perks: Vec<Id<Perk>>,
    /// If set, the NPC spawns with exactly these items. If `None`,
    /// gear is rolled from the faction archetype at the resolved rank.
    #[serde(default)]
    pub loadout: Option<Vec<Id<Item>>>,
    /// Only one instance of this NPC can exist at a time.
    #[serde(default = "default_true")]
    pub unique: bool,
    /// If killed, this NPC can be spawned again by future quests.
    /// If false, death is permanent.
    #[serde(default)]
    pub respawnable: bool,
}

fn default_true() -> bool {
    true
}

/// What role an employed NPC fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub enum Role {
    /// Goes into the Zone to scavenge, deliver, or gather intel.
    Runner,
    /// Stays at the bunker to deter theft, enable intimidation,
    /// and fight raids.
    Guard,
}

/// Core personality trait affecting negotiation behavior
/// (hidden from the player). Stored on entities as a field of
/// `NpcAttributes`, not as its own component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Personality {
    /// Careful, slow to trust, thorough negotiator.
    #[default]
    Cautious,
    /// Confrontational, may escalate if refused.
    Aggressive,
    /// Straightforward, unlikely to scam.
    Honest,
    /// May lie about item quality or their situation.
    Deceptive,
    /// Willing to go back and forth on price.
    Patient,
    /// Makes snap decisions, may accept bad deals.
    Impulsive,
}
