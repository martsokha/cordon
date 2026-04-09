//! Area definitions loaded from config.
//!
//! Each area is a point of interest on the Zone map: a location
//! with an area of effect. The [`id`](AreaDef::id) doubles as the
//! localization key.
//!
//! Areas come in five archetypes via [`AreaKind`]. Each archetype
//! carries the fields that make sense for it — Settlements have a
//! controlling faction and no danger (threat is a function of
//! faction relations at runtime), AnomalyFields and Anchors have
//! hazards, and so on.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Faction;
use crate::primitive::{Distance, HazardType, Id, IdMarker, Location, Tier};

/// Marker for area (point of interest) IDs.
pub struct Area;
impl IdMarker for Area {}

/// An environmental hazard with a type and intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hazard {
    /// What kind of hazard.
    pub kind: HazardType,
    /// How severe it is.
    pub intensity: Tier,
}

/// Role a settlement plays for its faction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementRole {
    /// A garrisoned position: barracks, watchtower, supply depot.
    Outpost,
    /// A trading hub where the player's runners can buy and sell.
    Market,
}

/// What kind of area this is. Each variant carries the fields that
/// matter for that archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AreaKind {
    /// Faction-held position. Threat is a function of faction
    /// relations at runtime, not a fixed danger profile — a
    /// friendly settlement is safe to walk into, an enemy one is
    /// hostile because the NPCs inside will attack.
    Settlement {
        faction: Id<Faction>,
        role: SettlementRole,
    },
    /// Unaffiliated open ground with light loot scatter.
    Wasteland { radiation: Tier, loot: Tier },
    /// Mutant-held area. Aggressive creatures, minor loot from
    /// what they've dragged in. (Reserved for the upcoming
    /// mutants faction — no current data uses this variant.)
    MutantLair { creatures: Tier, loot: Tier },
    /// Open hazard zone — chemical pools, electric storms, etc.
    /// Spawns relics whose origin matches the hazard kind.
    AnomalyField {
        hazard: Hazard,
        creatures: Tier,
        radiation: Tier,
        loot: Tier,
    },
    /// A pre-existing hardened structure (vault, bunker, archive)
    /// repurposed by survivors of the Zone. High loot, dense
    /// hazard, often guarded by mutants. Spawns relics like
    /// AnomalyField does.
    Anchor {
        hazard: Hazard,
        creatures: Tier,
        radiation: Tier,
        loot: Tier,
    },
}

impl AreaKind {
    /// Hazard for archetypes that have one (AnomalyField, Anchor).
    pub fn hazard(&self) -> Option<Hazard> {
        match self {
            AreaKind::AnomalyField { hazard, .. } | AreaKind::Anchor { hazard, .. } => {
                Some(*hazard)
            }
            _ => None,
        }
    }

    /// Creature density tier, where it applies.
    pub fn creatures(&self) -> Option<Tier> {
        match self {
            AreaKind::MutantLair { creatures, .. }
            | AreaKind::AnomalyField { creatures, .. }
            | AreaKind::Anchor { creatures, .. } => Some(*creatures),
            _ => None,
        }
    }

    /// Radiation tier, where it applies.
    pub fn radiation(&self) -> Option<Tier> {
        match self {
            AreaKind::Wasteland { radiation, .. }
            | AreaKind::AnomalyField { radiation, .. }
            | AreaKind::Anchor { radiation, .. } => Some(*radiation),
            _ => None,
        }
    }

    /// Loot tier, where it applies.
    pub fn loot(&self) -> Option<Tier> {
        match self {
            AreaKind::Wasteland { loot, .. }
            | AreaKind::MutantLair { loot, .. }
            | AreaKind::AnomalyField { loot, .. }
            | AreaKind::Anchor { loot, .. } => Some(*loot),
            _ => None,
        }
    }

    /// Faction that controls this area, if any. Only Settlements
    /// have a controlling faction.
    pub fn faction(&self) -> Option<&Id<Faction>> {
        match self {
            AreaKind::Settlement { faction, .. } => Some(faction),
            _ => None,
        }
    }
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
    /// Archetype-specific data.
    pub kind: AreaKind,
}
