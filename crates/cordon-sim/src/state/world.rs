use std::collections::HashMap;

use cordon_core::bunker::BunkerState;
use cordon_core::economy::mission::ActiveMission;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::player::PlayerState;
use cordon_core::object::id::{Id, Uid};
use cordon_core::world::event::Event;
use cordon_core::world::time::GameTime;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::state::market::MarketState;
use crate::state::sectors::SectorState;

/// The full mutable world state for a game session.
///
/// Created once at game start, mutated by the simulation each phase.
/// The game/UI layer reads this to render and submits player actions
/// that mutate it through simulation functions.
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
    pub rng: StdRng,
    /// All faction IDs from config (for random selection).
    pub faction_ids: Vec<Id>,
    next_uid: u32,
}

impl World {
    /// Create a new world with the given RNG seed and IDs from config.
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
            rng: StdRng::seed_from_u64(seed),
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

    /// Pick a random faction ID from the loaded config.
    pub fn random_faction(&mut self) -> Id {
        let idx = self.rng.gen_range(0..self.faction_ids.len());
        self.faction_ids[idx].clone()
    }
}
