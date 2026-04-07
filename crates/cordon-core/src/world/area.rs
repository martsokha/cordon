//! Area definitions loaded from config.
//!
//! Each area is a point of interest on the Zone map: a location
//! with an area of effect. The [`id`](AreaDef::id) doubles as the
//! localization key.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Faction;
use crate::primitive::{Distance, Environment, HazardType, Id, IdMarker, Location, Tier};

/// Marker for area (point of interest) IDs.
pub struct Area;
impl IdMarker for Area {}

/// How dangerous an area is across different dimensions.
///
/// Each dimension uses a [`Tier`] rating. The optional hazard
/// combines a type and intensity — an area can have at most one
/// dominant environmental hazard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DangerProfile {
    /// Creature density and aggression.
    pub creatures: Tier,
    /// Ambient radiation level.
    pub radiation: Tier,
    /// Dominant environmental hazard and its intensity.
    /// `None` means no environmental hazard beyond radiation/creatures.
    pub hazard: Option<Hazard>,
}

/// An environmental hazard with a type and intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hazard {
    /// What kind of hazard.
    pub kind: HazardType,
    /// How severe it is.
    pub intensity: Tier,
}

/// An area of the Zone, loaded from config.
///
/// Areas are points of interest on the map, defined by a center
/// [`location`](AreaDef::location) and an influence
/// [`radius`](AreaDef::radius). Runners travel freely across the
/// map — travel time depends on distance from the bunker.
///
/// Live state (faction control, creature activity, etc.) is tracked
/// separately by the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaDef {
    /// Unique identifier and localization key (e.g., `"cordon"`, `"dead_city"`).
    pub id: Id<Area>,
    /// Center position on the Zone map.
    pub location: Location,
    /// Radius of the area's influence.
    pub radius: Distance,
    /// Indoor, outdoor, or underground.
    pub environment: Environment,
    /// Base danger across different dimensions.
    pub danger: DangerProfile,
    /// Loot quality tier for this area.
    pub loot_tier: Tier,
    /// Faction ID that controls this area by default, if any.
    pub default_faction: Option<Id<Faction>>,
}
