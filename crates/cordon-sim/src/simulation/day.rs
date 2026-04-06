use std::collections::HashMap;

use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NamePool;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::{Id, Period};
use cordon_core::world::event::EventDef;
use cordon_core::world::mission::MissionResult;

use crate::simulation::npcs::NpcGenerator;
use crate::simulation::{events, factions, missions, npcs};
use crate::state::world::World;

/// Results from advancing a period, for the game layer to consume.
pub enum PeriodResult {
    Working {
        visitors: Vec<Npc>,
        events_started: usize,
    },
    Off {
        mission_results: Vec<MissionResult>,
    },
}

/// Advance the world by one period. Returns what happened for the UI to render.
pub fn advance_period(
    world: &mut World,
    event_defs: &[EventDef],
    npc_gen: &impl NpcGenerator,
    name_pools: &HashMap<Id<Faction>, NamePool>,
    fallback_pool: &NamePool,
) -> PeriodResult {
    match world.time.period {
        Period::Working => {
            let event_count_before = world.active_events.len();
            events::roll_daily_events(world, event_defs);
            let events_started = world.active_events.len() - event_count_before;

            let visitors = npcs::spawn_daily_visitors(world, npc_gen, name_pools, fallback_pool);
            factions::tick_factions(world);

            world.time.advance();

            PeriodResult::Working {
                visitors,
                events_started,
            }
        }
        Period::Off => {
            let mission_results = missions::resolve_missions(world);
            events::expire_events(world);
            world.time.advance();
            PeriodResult::Off { mission_results }
        }
    }
}
