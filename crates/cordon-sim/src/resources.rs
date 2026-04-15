//! Top-level Bevy resources owned by `cordon-sim`, plus the
//! world-bootstrap function that populates them.
//!
//! Each concern is its own resource so systems declare exactly
//! what they touch and Bevy can run them in parallel where
//! possible. [`init_world_resources`] is called once by the
//! cordon-bevy layer on `OnEnter(AppState::Playing)` and fills
//! every resource defined below from loaded [`GameDataResource`].

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::player::PlayerState;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::{GameTime, Id, Uid};
use cordon_core::world::area::{Area, AreaKind};
use cordon_core::world::narrative::ActiveEvent;
use cordon_data::gamedata::GameDataResource;

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

/// Per-hire bookkeeping for one squad on the player's roster.
///
/// Empty for now — the field exists so future per-hire metadata
/// (date hired, custom callsign, assignment) can be added without
/// changing [`PlayerSquadRoster`]'s shape.
///
/// Daily pay is intentionally **not** stored here — it's a pure
/// function of the squad's current member ranks (see
/// [`Rank::pay`](cordon_core::primitive::Rank::pay)) so member
/// deaths immediately reduce the bill with no recompute system.
#[derive(Debug, Clone, Default)]
pub struct PlayerSquadEntry {}

/// All squads the player has hired. Squads are the only unit of
/// player ownership — there is no individual NPC hiring.
///
/// Keyed by stable [`Uid<Squad>`] so the roster survives respawns
/// and is save-ready. ECS systems that need entity access find
/// the live entity via [`SquadIdIndex`] (or, more commonly, through
/// the derived [`Owned`](crate::behavior::squad::Owned) marker
/// which is kept in sync by [`sync_owned_marker`]).
#[derive(Resource, Default, Debug, Clone)]
pub struct PlayerSquadRoster {
    entries: HashMap<Uid<Squad>, PlayerSquadEntry>,
}

impl PlayerSquadRoster {
    /// Add a squad to the roster. No-op if already hired.
    pub fn hire(&mut self, squad: Uid<Squad>) {
        self.entries.entry(squad).or_default();
    }

    /// Remove a squad from the roster. No-op if not hired.
    pub fn dismiss(&mut self, squad: &Uid<Squad>) {
        self.entries.remove(squad);
    }

    pub fn is_hired(&self, squad: &Uid<Squad>) -> bool {
        self.entries.contains_key(squad)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Uid<Squad>, &PlayerSquadEntry)> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// In-game clock. Advanced by `cordon_bevy::world::tick_game_time`.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GameClock(pub GameTime);

/// Player state: credits, XP, faction standings, hired NPCs, bunker
/// upgrades and storage. Mutated by faction reactions to events and
/// (later) by player commands.
#[derive(Resource, Debug, Clone)]
pub struct Player(pub PlayerState);

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

/// Per-frame fractional accumulator for [`tick_game_time`]. Keeps
/// sub-minute progress between frames so the clock doesn't
/// discretely jump whenever a whole minute happens to align with
/// a frame boundary.
#[derive(Resource, Default, Debug)]
pub struct TimeAccumulator(pub f32);

/// How many game minutes pass per real second at 1× time scale.
/// A game day at this rate is 12 real minutes; the F4 debug cheat
/// in cordon-bevy multiplies this via `Time<Virtual>`.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

/// Build the cordon-sim resource set from loaded game data. The
/// caller is responsible for calling this exactly once, typically
/// on `OnEnter(PlayingState)` in the cordon-bevy layer.
pub fn init_world_resources(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;

    let faction_ids = data.faction_ids();
    // Pair each faction with its spawn weight from config so the
    // spawn system can do a weighted pick without re-reading the
    // FactionDef catalog every wave.
    let faction_weights: Vec<(_, u32)> = faction_ids
        .iter()
        .map(|id| {
            let weight = data.factions.get(id).map(|f| f.spawn_weight).unwrap_or(1);
            (id.clone(), weight)
        })
        .collect();

    let mut areas: HashMap<_, _> = HashMap::with_capacity(data.areas.len());
    for id in data.area_ids() {
        areas.insert(id.clone(), AreaState::new(id.clone()));
    }

    // Pre-collect each faction's Settlement centres so the spawn
    // system doesn't have to walk every area every wave. Built once
    // here because settlements are static config — they don't
    // change at runtime.
    let mut settlements: HashMap<_, Vec<Vec2>> = HashMap::with_capacity(faction_ids.len());
    for area in data.areas.values() {
        if let AreaKind::Settlement { faction, .. } = &area.kind {
            settlements
                .entry(faction.clone())
                .or_default()
                .push(Vec2::new(area.location.x, area.location.y));
        }
    }

    commands.insert_resource(GameClock::default());
    commands.insert_resource(Player(PlayerState::new(&faction_ids)));
    commands.insert_resource(PlayerSquadRoster::default());
    commands.insert_resource(FactionIndex(faction_weights));
    commands.insert_resource(FactionSettlements(settlements));
    commands.insert_resource(AreaStates(areas));
    commands.insert_resource(EventLog::default());
    commands.insert_resource(TimeAccumulator::default());

    info!("World initialised; population will be spawned by cordon-sim");
}

/// Per-frame clock advance. Reads `Res<Time>` (which is virtual
/// time by default in Bevy 0.18), so time-scale cheats applied
/// via `Time<Virtual>::set_relative_speed` naturally accelerate
/// the game clock along with the rest of the sim.
pub fn tick_game_time(
    time: Res<Time>,
    mut acc: ResMut<TimeAccumulator>,
    mut clock: ResMut<GameClock>,
) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}
