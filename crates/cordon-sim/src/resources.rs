//! Top-level Bevy resources owned by `cordon-sim`.
//!
//! Each concern is its own resource so systems declare exactly what
//! they touch and Bevy can run them in parallel where possible.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::player::PlayerState;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::{GameTime, Id, Uid};
use cordon_core::world::area::Area;
use cordon_core::world::event::ActiveEvent;

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

/// Maps stable squad uids to their current ECS entity. Maintained by
/// the spawn system and used by AI systems for the rare uid → entity
/// lookups (e.g. resolving `Goal::Protect { other }`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SquadIdIndex(pub HashMap<Uid<Squad>, Entity>);

/// In-game clock. Advanced by `cordon_bevy::world::tick_game_time`.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GameClock(pub GameTime);

/// Player state: credits, XP, faction standings, hired NPCs, bunker
/// upgrades and storage. Mutated by faction reactions to events and
/// (later) by player commands.
#[derive(Resource, Debug, Clone)]
pub struct Player(pub PlayerState);

/// All faction IDs from config, used for random faction selection
/// during NPC generation. Built once at world init from
/// `GameDataResource`.
#[derive(Resource, Debug, Clone, Default)]
pub struct FactionIndex(pub Vec<Id<Faction>>);

/// Live area states keyed by area id. Tracks faction control, danger,
/// creature activity.
#[derive(Resource, Default)]
pub struct AreaStates(pub HashMap<Id<Area>, AreaState>);

/// All currently-active environmental/economic/faction/personal
/// events. Rolled daily; expired entries pruned at the day rollover.
#[derive(Resource, Debug, Clone, Default)]
pub struct EventLog(pub Vec<ActiveEvent>);

/// Monotonic Uid allocator. Each call to [`UidAllocator::alloc`]
/// returns a fresh `Uid<T>` typed for the caller's marker.
#[derive(Resource, Debug, Clone)]
pub struct UidAllocator {
    next: u32,
}

impl Default for UidAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl UidAllocator {
    pub fn alloc<T: Send + Sync + 'static>(&mut self) -> Uid<T> {
        let uid = Uid::new(self.next);
        self.next += 1;
        uid
    }
}
