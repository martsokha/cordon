//! Bunker state, upgrade definitions, and storage.
//!
//! The bunker is the player's base of operations. Upgrades are either
//! purchasable (available if you have credits and prerequisites) or
//! faction-gated (require standing or quest completion).

use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::primitive::id::Id;

/// How an upgrade becomes available to the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpgradeSource {
    /// Always available for purchase if prerequisites are met.
    /// Basic bunker improvements: fridge, generator, cot, etc.
    Purchasable,
    /// Offered by a specific faction once standing is high enough.
    Faction {
        /// Faction ID that offers this upgrade.
        faction: Id,
        /// Minimum standing required.
        min_standing: i8,
    },
    /// Rewarded for completing a specific quest or mission.
    /// Not purchasable — earned through gameplay.
    Quest,
}

/// An upgrade definition loaded from config.
///
/// All upgrades live in a flat list with prerequisite references.
/// There are no hardcoded "chains" — sequential upgrades like
/// `"radio_1"` → `"radio_2"` → `"radio_3"` are modeled by each
/// upgrade requiring the previous one in its [`requires`](UpgradeDef::requires) list.
///
/// The [`id`](UpgradeDef::id) doubles as the localization key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeDef {
    /// Unique identifier and localization key (e.g., `"fridge"`, `"radio_3"`).
    pub id: Id,
    /// Credit cost to purchase. Zero for quest rewards.
    pub cost: u32,
    /// IDs of other upgrades that must be installed first.
    pub requires: Vec<Id>,
    /// How this upgrade becomes available.
    pub source: UpgradeSource,
}

/// The bunker's current state.
///
/// Tracks which upgrades are installed and the contents of storage.
/// All upgrade effects are derived from the set of installed upgrade
/// IDs — the sim checks `has_upgrade("radio_3")` rather than reading
/// a chain level number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunkerState {
    /// All installed upgrade IDs.
    pub upgrades: Vec<Id>,
    /// Main storage contents.
    pub storage: Vec<Item>,
    /// Hidden storage contents (survives raids, invisible during inspections).
    pub hidden_storage: Vec<Item>,
}

impl BunkerState {
    /// Create a new empty bunker with no upgrades.
    pub fn new() -> Self {
        Self {
            upgrades: Vec::new(),
            storage: Vec::new(),
            hidden_storage: Vec::new(),
        }
    }

    /// Check if an upgrade is installed.
    pub fn has_upgrade(&self, upgrade_id: &Id) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Whether the bunker has a generator (prevents power outages).
    pub fn has_power(&self) -> bool {
        self.has_upgrade(&Id::new("generator"))
    }
}

impl Default for BunkerState {
    fn default() -> Self {
        Self::new()
    }
}
