//! World snapshots built from game data and mutated by world
//! systems: per-area runtime state, faction weighting for
//! spawning, settlement positions, active events.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::area::Area;
use cordon_core::world::narrative::ActiveEvent;

/// Live state of an area in the world.
///
/// Tracks dynamic properties that change during gameplay: faction
/// control, danger modifiers from events, creature activity. Base
/// danger/reward values come from the area's config definition.
pub struct AreaState {
    pub id: Id<Area>,
    /// Which faction currently controls this area, if any.
    pub controlling_faction: Option<Id<Faction>>,
    /// Additive danger modifier from events/world state.
    pub danger_modifier: f32,
    /// Creature activity level (0.0–1.0). Affects danger.
    pub creature_activity: f32,
    /// Whether a hazard field is currently active.
    pub hazard_active: bool,
}

impl AreaState {
    pub fn new(id: Id<Area>) -> Self {
        Self {
            id,
            controlling_faction: None,
            danger_modifier: 0.0,
            creature_activity: 0.0,
            hazard_active: false,
        }
    }
}

/// All faction IDs from config paired with their spawn weight, used
/// for weighted faction selection during NPC generation. Built once
/// at world init from `GameDataResource`. The [`FactionDef`] field
/// `spawn_weight` controls how often each faction is rolled.
#[derive(Resource, Debug, Clone, Default)]
pub struct FactionIndex(pub Vec<(Id<Faction>, u32)>);

/// Pre-collected centres of every Settlement-archetype area, indexed
/// by controlling faction. Built once at world init so the spawn
/// system doesn't have to walk every area every wave to figure out
/// where a faction's bases are.
#[derive(Resource, Debug, Clone, Default)]
pub struct FactionSettlements(pub HashMap<Id<Faction>, Vec<bevy::math::Vec2>>);

/// Live area states keyed by area id. Tracks faction control, danger,
/// creature activity.
#[derive(Resource, Default)]
pub struct AreaStates(pub HashMap<Id<Area>, AreaState>);

/// All currently-active environmental/economic/faction/personal
/// events. Rolled daily; expired entries pruned at the day rollover.
#[derive(Resource, Debug, Clone, Default)]
pub struct EventLog(pub Vec<ActiveEvent>);
