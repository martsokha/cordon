use std::collections::HashMap;

use cordon_core::bunker::BunkerState;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::player::PlayerState;
use cordon_core::primitive::id::{Id, Uid};
use cordon_core::world::event::Event;
use cordon_core::world::mission::ActiveMission;
use cordon_core::world::time::GameTime;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::state::market::MarketState;
use crate::state::sectors::SectorState;

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

impl SimRng {
    /// Derive per-subsystem RNGs from a master seed.
    ///
    /// Each subsystem gets a unique seed by XOR-ing the master seed
    /// with a different constant, ensuring independent sequences.
    pub fn new(seed: u64) -> Self {
        Self {
            events: StdRng::seed_from_u64(seed ^ 0x4556454E54530001),
            npcs: StdRng::seed_from_u64(seed ^ 0x4E50435300000002),
            missions: StdRng::seed_from_u64(seed ^ 0x4D495353494F4E03),
            market: StdRng::seed_from_u64(seed ^ 0x4D41524B45540004),
            factions: StdRng::seed_from_u64(seed ^ 0x464143544E530005),
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
    pub bunker: BunkerState,
    /// Live sector states keyed by sector ID.
    pub sectors: HashMap<Id, SectorState>,
    /// All NPCs in the world keyed by runtime UID.
    pub npcs: HashMap<Uid, Npc>,
    pub active_events: Vec<Event>,
    pub active_missions: Vec<ActiveMission>,
    pub market: MarketState,
    /// Per-subsystem deterministic RNGs.
    pub rng: SimRng,
    /// All faction IDs from config (for random selection).
    pub faction_ids: Vec<Id>,
    next_uid: u32,
}

impl World {
    /// Create a new world with the given RNG seed and IDs from config.
    ///
    /// The seed determines the entire game session. All randomness
    /// is derived from it through per-subsystem RNGs.
    pub fn new(seed: u64, faction_ids: Vec<Id>, sector_ids: &[Id]) -> Self {
        let mut sectors = HashMap::new();
        for id in sector_ids {
            sectors.insert(id.clone(), SectorState::new(id.clone()));
        }

        let player = PlayerState::new(&faction_ids);

        Self {
            time: GameTime::new(),
            player,
            bunker: BunkerState::new(),
            sectors,
            npcs: HashMap::new(),
            active_events: Vec::new(),
            active_missions: Vec::new(),
            market: MarketState::new(),
            rng: SimRng::new(seed),
            faction_ids,
            next_uid: 1,
        }
    }

    /// Allocate a unique runtime ID for NPCs and missions.
    pub fn alloc_uid(&mut self) -> Uid {
        let uid = Uid(self.next_uid);
        self.next_uid += 1;
        uid
    }

    /// Current game day.
    pub fn current_day(&self) -> cordon_core::world::time::Day {
        self.time.day
    }

    /// Pick a random faction ID using the NPC subsystem RNG.
    pub fn random_faction(&mut self) -> Id {
        let idx = self.rng.npcs.gen_range(0..self.faction_ids.len());
        self.faction_ids[idx].clone()
    }
}
