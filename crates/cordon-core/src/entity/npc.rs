//! NPC type markers.
//!
//! There's no `Npc` data struct anymore — NPCs are Bevy entities
//! assembled from cordon-core component types (`NpcName`,
//! `Loadout`, `Experience`, `Credits`) plus cordon-sim glue
//! (`NpcMarker`, `FactionId`, `Employment`, `NpcBundle`). The
//! generator in cordon-sim produces bundles directly.
//!
//! What remains here is:
//!
//! - `Npc` — a phantom marker type used as the type parameter
//!   on [`Uid<Npc>`], so stable save-game IDs stay typed.
//! - `NpcTemplate` — marker for NPC template IDs used in quest
//!   consequences.

use serde::{Deserialize, Serialize};

use super::faction::Faction;
use crate::item::Item;
use crate::primitive::{Id, IdMarker, Rank};

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
/// identities — "Sergeant Petrov," not "a random Garrison soldier."
/// Generic NPCs are spawned from [`ArchetypeDef`](super::archetype::ArchetypeDef)
/// instead.
///
/// At spawn time the template's rank determines base stats, and gear
/// is either pulled from the faction's archetype pool (if `loadout`
/// is `None`) or set to the authored item list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplateDef {
    pub id: Id<NpcTemplate>,
    pub faction: Id<Faction>,
    /// Base rank — actual spawn XP is randomized within this tier.
    pub rank: Rank,
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

impl NpcTemplateDef {
    /// Localization key for the display name, derived from the
    /// template id by Fluent-casing (`_` → `-`). The name
    /// convention is stable across the codebase — see the
    /// `npc-*` keys in `names.ftl`.
    pub fn name_key(&self) -> String {
        self.id.as_str().replace('_', "-")
    }
}

fn default_true() -> bool {
    true
}
