//! Sector definitions loaded from config.
//!
//! Each sector is a point of interest on the Zone map: a location
//! with an area of effect. The [`id`](SectorDef::id) doubles as the
//! localization key.

use serde::{Deserialize, Serialize};

use crate::primitive::id::Id;
use crate::primitive::location::Location;

/// A sector of the Zone, loaded from config.
///
/// Sectors are points of interest on the map, defined by a center
/// [`location`](SectorDef::location) and an area
/// [`radius`](SectorDef::radius). Runners travel to the sector's
/// location — travel time depends on distance from the bunker.
///
/// Live state (faction control, creature activity, etc.) is tracked
/// separately by the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorDef {
    /// Unique identifier and localization key (e.g., `"threshold"`, `"core"`).
    pub id: Id,
    /// Center position on the Zone map.
    pub location: Location,
    /// Radius of the sector's area of influence in map units.
    pub radius: f32,
    /// Base danger level (0.0–1.0). Modified at runtime by events.
    pub base_danger: f32,
    /// Base reward quality (0.0–1.0). Affects loot table selection.
    pub base_reward: f32,
    /// Upgrade ID required to send runners to this sector (e.g., `"radio_3"`).
    /// `None` means no upgrade required — the sector is always reachable.
    pub required_upgrade: Option<Id>,
    /// Faction ID that controls this sector by default, if any.
    pub default_faction: Option<Id>,
}
