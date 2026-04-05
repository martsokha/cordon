//! Area definitions loaded from config.
//!
//! Each area is a point of interest on the Zone map: a location
//! with an area of effect. The [`id`](AreaDef::id) doubles as the
//! localization key.

use serde::{Deserialize, Serialize};

use crate::entity::bunker::Upgrade;
use crate::entity::faction::Faction;
use crate::primitive::id::{Id, IdMarker};
use crate::primitive::location::Location;

/// Marker for area (point of interest) IDs.
pub struct Area;
impl IdMarker for Area {}

/// An area of the Zone, loaded from config.
///
/// Areas are points of interest on the map, defined by a center
/// [`location`](AreaDef::location) and an area
/// [`radius`](AreaDef::radius). Runners travel to the area's
/// location — travel time depends on distance from the bunker.
///
/// Live state (faction control, creature activity, etc.) is tracked
/// separately by the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDef {
    /// Unique identifier and localization key (e.g., `"threshold"`, `"core"`).
    pub id: Id<Area>,
    /// Center position on the Zone map.
    pub location: Location,
    /// Radius of the area's area of influence in map units.
    pub radius: f32,
    /// Base danger level (0.0–1.0). Modified at runtime by events.
    pub base_danger: f32,
    /// Base reward quality (0.0–1.0). Affects loot table selection.
    pub base_reward: f32,
    /// Upgrade ID required to send runners to this area (e.g., `"radio_3"`).
    /// `None` means no upgrade required — the area is always reachable.
    pub required_upgrade: Option<Id<Upgrade>>,
    /// Faction ID that controls this area by default, if any.
    pub default_faction: Option<Id<Faction>>,
}
