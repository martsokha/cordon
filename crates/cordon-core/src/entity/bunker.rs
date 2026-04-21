//! Bunker upgrade definitions.
//!
//! The player operates from two connected locations:
//! - **Bunker**: the interior where trading happens. Storage, counter,
//!   laptop, and indoor upgrades live here.
//! - **Camp**: the area surrounding the bunker. Defenses, antenna,
//!   outdoor structures, and camp-wide upgrades live here.
//!
//! Upgrades are either purchasable, faction-gated, or quest rewards.
//! Runtime state for which upgrades are *installed* lives on
//! [`PlayerState`](super::player::PlayerState); this module only owns
//! the static def types.

use serde::{Deserialize, Serialize};

use super::faction::Faction;
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

/// What an installed upgrade *does*. An upgrade carries a list of
/// these; systems query installed upgrades for effects they care
/// about rather than matching on upgrade IDs.
///
/// Add variants as new mechanical categories of upgrade surface.
/// Flavour-only upgrades (cosmetic props, wallpaper) get an empty
/// effects list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpgradeEffect {
    /// Spawn a pair of storage racks in the hall. Multiple
    /// `HallRackPair` effects each add another pair (one upgrade
    /// → north pair, a second → south pair). Consumed by the
    /// bunker's `hall` room spawner.
    HallRackPair,
    /// Reveal relic markers on the zone map regardless of NPC
    /// vision / fog-of-war. Consumed by the laptop's fog system.
    RevealRelics,
    /// Decrypt encrypted radio broadcasts. Events whose `RadioEntry`
    /// is marked `encrypted` only reach the player when at least
    /// one installed upgrade grants this effect. Also gates the
    /// `Radio04` chunky tube-radio prop in the command room.
    ListeningDevice,
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
    /// What this upgrade does once installed. Empty for flavour-
    /// only upgrades; otherwise a list of [`UpgradeEffect`]s that
    /// downstream systems query for.
    #[serde(default)]
    pub effects: Vec<UpgradeEffect>,
}
