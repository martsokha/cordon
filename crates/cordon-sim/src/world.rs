use std::collections::HashMap;

use cordon_core::bunker::BunkerState;
use cordon_core::event::Event;
use cordon_core::mission::ActiveMission;
use cordon_core::npc::{Npc, NpcId};
use cordon_core::player::PlayerState;
use cordon_core::sector::SectorId;
use cordon_core::time::GameTime;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::market::MarketState;
use crate::sectors::SectorState;

pub struct World {
    pub time: GameTime,
    pub player: PlayerState,
    pub bunker: BunkerState,
    pub sectors: HashMap<SectorId, SectorState>,
    pub npcs: HashMap<NpcId, Npc>,
    pub active_events: Vec<Event>,
    pub active_missions: Vec<ActiveMission>,
    pub market: MarketState,
    pub rng: StdRng,
    pub next_npc_id: u32,
    pub next_mission_id: u32,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut sectors = HashMap::new();
        for &id in &SectorId::ALL {
            sectors.insert(id, SectorState::new(id));
        }

        Self {
            time: GameTime::new(),
            player: PlayerState::new(),
            bunker: BunkerState::new(),
            sectors,
            npcs: HashMap::new(),
            active_events: Vec::new(),
            active_missions: Vec::new(),
            market: MarketState::new(),
            rng: StdRng::seed_from_u64(seed),
            next_npc_id: 1,
            next_mission_id: 1,
        }
    }

    pub fn alloc_npc_id(&mut self) -> NpcId {
        let id = NpcId(self.next_npc_id);
        self.next_npc_id += 1;
        id
    }

    pub fn alloc_mission_id(&mut self) -> cordon_core::mission::MissionId {
        let id = cordon_core::mission::MissionId(self.next_mission_id);
        self.next_mission_id += 1;
        id
    }

    pub fn current_day(&self) -> cordon_core::time::Day {
        self.time.day
    }
}
