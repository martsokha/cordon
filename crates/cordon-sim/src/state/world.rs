use std::collections::HashMap;

use cordon_core::entity::bunker::BaseState;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::player::PlayerState;
use cordon_core::primitive::id::Id;
use cordon_core::primitive::time::GameTime;
use cordon_core::primitive::uid::Uid;
use cordon_core::world::area::Area;
use cordon_core::world::event::ActiveEvent;
use cordon_core::world::mission::ActiveMission;
use cordon_core::world::narrative::quest::{ActiveQuest, CompletedQuest};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use crate::state::market::MarketState;
use crate::state::sectors::AreaState;

/// Per-subsystem RNGs derived from the master seed.
///
/// Each subsystem gets its own RNG so that adding a roll to one
/// subsystem doesn't shift the sequence of another. This makes
/// the simulation fully deterministic and stable across code changes
/// within a subsystem.
pub struct SimRng {
    /// RNG for event scheduling.
    pub events: StdRng,
    /// RNG for NPC generation and behavior.
    pub npcs: StdRng,
    /// RNG for mission outcome resolution.
    pub missions: StdRng,
    /// RNG for market fluctuations.
    pub market: StdRng,
    /// RNG for faction dynamics.
    pub factions: StdRng,
}

/// Salt values for deriving per-subsystem seeds from the master seed.
///
/// Each subsystem XORs the master seed with its salt to get an
/// independent RNG sequence. Changing a salt replays that subsystem
/// differently without affecting others.
struct SimRngSalts {
    events: u64,
    npcs: u64,
    missions: u64,
    market: u64,
    factions: u64,
}

const SALTS: SimRngSalts = SimRngSalts {
    events: 0x4556454E54530001,
    npcs: 0x4E50435300000002,
    missions: 0x4D495353494F4E03,
    market: 0x4D41524B45540004,
    factions: 0x464143544E530005,
};

impl SimRng {
    /// Derive per-subsystem RNGs from a master seed.
    pub fn new(seed: u64) -> Self {
        Self {
            events: StdRng::seed_from_u64(seed ^ SALTS.events),
            npcs: StdRng::seed_from_u64(seed ^ SALTS.npcs),
            missions: StdRng::seed_from_u64(seed ^ SALTS.missions),
            market: StdRng::seed_from_u64(seed ^ SALTS.market),
            factions: StdRng::seed_from_u64(seed ^ SALTS.factions),
        }
    }
}

/// The full mutable world state for a game session.
///
/// Created once at game start from a single seed. The simulation is
/// fully deterministic: given the same seed and the same player
/// actions, the world will evolve identically.
pub struct World {
    pub time: GameTime,
    pub player: PlayerState,
    pub bunker: BaseState,
    /// Live area states keyed by area ID.
    pub areas: HashMap<Id<Area>, AreaState>,
    /// All NPCs in the world keyed by runtime UID.
    pub npcs: HashMap<Uid<Npc>, Npc>,
    pub active_events: Vec<ActiveEvent>,
    pub active_missions: Vec<ActiveMission>,
    /// Quests currently in progress.
    pub active_quests: Vec<ActiveQuest>,
    /// Quests that have been completed (for prerequisite checking).
    pub completed_quests: Vec<CompletedQuest>,
    pub market: MarketState,
    /// Per-subsystem deterministic RNGs.
    pub rng: SimRng,
    /// All faction IDs from config (for random selection).
    pub faction_ids: Vec<Id<Faction>>,
    next_uid: u32,
}

impl World {
    /// Create a new world with the given RNG seed and IDs from config.
    ///
    /// The seed determines the entire game session. All randomness
    /// is derived from it through per-subsystem RNGs.
    pub fn new(seed: u64, faction_ids: Vec<Id<Faction>>, sector_ids: &[Id<Area>]) -> Self {
        let mut areas = HashMap::new();
        for id in sector_ids {
            areas.insert(id.clone(), AreaState::new(id.clone()));
        }

        let player = PlayerState::new(&faction_ids);

        Self {
            time: GameTime::new(),
            player,
            bunker: BaseState::new(),
            areas,
            npcs: HashMap::new(),
            active_events: Vec::new(),
            active_missions: Vec::new(),
            active_quests: Vec::new(),
            completed_quests: Vec::new(),
            market: MarketState::new(),
            rng: SimRng::new(seed),
            faction_ids,
            next_uid: 1,
        }
    }

    /// Allocate a unique runtime ID for NPCs and missions.
    pub fn alloc_uid<T: 'static>(&mut self) -> Uid<T> {
        let uid = Uid::new(self.next_uid);
        self.next_uid += 1;
        uid
    }

    /// Current game day.
    pub fn current_day(&self) -> cordon_core::primitive::time::Day {
        self.time.day
    }

    /// Pick a random faction ID using the NPC subsystem RNG.
    pub fn random_faction(&mut self) -> Id<Faction> {
        let idx = self.rng.npcs.random_range(0..self.faction_ids.len());
        self.faction_ids[idx].clone()
    }
}
