use cordon_core::entity::npc::Npc;
use cordon_core::world::event::EventDef;
use cordon_core::world::mission::MissionResult;
use cordon_core::world::time::Phase;

use crate::simulation::{events, factions, missions, npcs};
use crate::state::world::World;

/// Results from advancing a phase, for the game layer to consume.
pub enum PhaseResult {
    Morning {
        visitors: Vec<Npc>,
        events_started: usize,
    },
    Midday,
    Evening {
        mission_results: Vec<MissionResult>,
    },
    Night,
}

/// Advance the world by one phase. Returns what happened for the UI to render.
///
/// Takes `event_defs` from the loaded [`GameData`](cordon_data::catalog::GameData)
/// to roll daily events.
pub fn advance_phase(world: &mut World, event_defs: &[EventDef]) -> PhaseResult {
    match world.time.phase {
        Phase::Morning => {
            let event_count_before = world.active_events.len();
            events::roll_daily_events(world, event_defs);
            let events_started = world.active_events.len() - event_count_before;

            let visitors = npcs::spawn_daily_visitors(world);
            factions::tick_factions(world);

            world.time.advance();

            PhaseResult::Morning {
                visitors,
                events_started,
            }
        }
        Phase::Midday => {
            world.time.advance();
            PhaseResult::Midday
        }
        Phase::Evening => {
            let mission_results = missions::resolve_missions(world);
            world.time.advance();
            PhaseResult::Evening { mission_results }
        }
        Phase::Night => {
            events::expire_events(world);
            // TODO: spoilage, relic degradation, payroll deduction
            world.time.advance();
            PhaseResult::Night
        }
    }
}
