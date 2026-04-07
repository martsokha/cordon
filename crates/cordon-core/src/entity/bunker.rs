//! Base state: bunker, camp, upgrades, and storage.
//!
//! The player operates from two connected locations:
//! - **Bunker**: the interior where trading happens. Storage, counter,
//!   laptop, and indoor upgrades live here.
//! - **Camp**: the area surrounding the bunker. Defenses, antenna,
//!   outdoor structures, and camp-wide upgrades live here.
//!
//! Upgrades are either purchasable, faction-gated, or quest rewards.
//! All upgrades are data-driven and reference a location.

use serde::{Deserialize, Serialize};

use super::faction::Faction;
use crate::item::Stash;
use crate::primitive::{Credits, Id, IdMarker, Relation};

/// Marker for upgrade IDs.
pub struct Upgrade;
impl IdMarker for Upgrade {}

/// Where an upgrade is physically installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UpgradeLocation {
    /// Inside the bunker: storage, counter, laptop, fridge, etc.
    Bunker,
    /// Outside in the camp: antenna, defenses, watchtower, etc.
    Camp,
}

/// How an upgrade becomes available to the player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpgradeSource {
    /// Always available for purchase if prerequisites are met.
    /// Basic improvements: fridge, generator, cot, etc.
    Purchasable,
    /// Offered by a specific faction once standing is high enough.
    Faction {
        /// Faction ID that offers this upgrade.
        faction: Id<Faction>,
        /// Minimum standing required.
        min_standing: Relation,
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
    /// Unique identifier and localization key (e.g., `"fridge"`, `"watchtower"`).
    pub id: Id<Upgrade>,
    /// Where this upgrade is installed.
    pub location: UpgradeLocation,
    /// Credit cost to purchase. Zero for quest rewards.
    pub cost: Credits,
    /// IDs of other upgrades that must be installed first.
    pub requires: Vec<Id<Upgrade>>,
    /// How this upgrade becomes available.
    pub source: UpgradeSource,
}

/// The player's base state: bunker interior + camp exterior.
///
/// Tracks which upgrades are installed and the contents of storage.
/// All upgrade effects are derived from the set of installed upgrade
/// IDs — the sim checks `has_upgrade("radio_3")` rather than reading
/// a level number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseState {
    /// All installed upgrade IDs (both bunker and camp).
    pub upgrades: Vec<Id<Upgrade>>,
    /// Main storage (bunker interior).
    pub storage: Stash,
    /// Hidden storage (survives raids, invisible during inspections).
    pub hidden_storage: Stash,
}

impl BaseState {
    /// Create a new empty base with default storage capacities.
    pub fn new() -> Self {
        Self {
            upgrades: Vec::new(),
            storage: Stash::new(20),
            hidden_storage: Stash::new(0),
        }
    }

    /// Check if an upgrade is installed (bunker or camp).
    pub fn has_upgrade(&self, upgrade_id: &Id<Upgrade>) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Whether the base has a generator (prevents power outages).
    pub fn has_power(&self) -> bool {
        self.has_upgrade(&Id::<Upgrade>::new("generator"))
    }
}

impl Default for BaseState {
    fn default() -> Self {
        Self::new()
    }
}
