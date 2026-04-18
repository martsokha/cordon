//! NPC type markers + the hidden-personality enum.
//!
//! There's no `Npc` data struct anymore — NPCs are Bevy entities
//! assembled from cordon-core component types (`NpcName`,
//! `Loadout`, `Experience`, `Credits`, `Trust`, `Loyalty`,
//! `Personality`) plus cordon-sim glue (`NpcMarker`,
//! `FactionId`, `NpcAttributes`, `Employment`,
//! `NpcBundle`). The generator in cordon-sim produces bundles
//! directly.
//!
//! What remains here is:
//!
//! - `Npc` — a phantom marker type used as the type parameter
//!   on [`Uid<Npc>`], so stable save-game IDs stay typed.
//! - `Personality` — enum flavour type stored as a field inside
//!   a component, not as its own component.
//! - `NpcTemplate` — marker for NPC template IDs used in quest
//!   consequences.

use serde::{Deserialize, Serialize};

use super::faction::Faction;
use crate::item::Item;
use crate::primitive::{Id, IdMarker, Rank, Trust};

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
    /// When true the NPC is excluded from the combat simulation
    /// entirely: squads won't target them, shots won't land, and
    /// the death system ignores them. Use for story-critical
    /// characters who must survive to fulfil their narrative role.
    #[serde(default)]
    pub essential: bool,
}

fn default_true() -> bool {
    true
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
