use cordon_core::economy::mission::MissionResult;
use cordon_core::entity::npc::Npc;
use cordon_core::world::time::Phase;

use crate::events;
use crate::factions;
use crate::missions;
use crate::npcs;
use crate::world::World;

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
pub fn advance_phase(world: &mut World) -> PhaseResult {
    match world.time.phase {
        Phase::Morning => {
            // Roll daily events
            let event_count_before = world.active_events.len();
            events::roll_daily_events(world);
            let events_started = world.active_events.len() - event_count_before;

            // Spawn visitors
            let visitors = npcs::spawn_daily_visitors(world);

            // Update factions
            factions::tick_factions(world);

            world.time.advance();

            PhaseResult::Morning {
                visitors,
                events_started,
            }
        }
        Phase::Midday => {
            // Trading phase: driven by the game/UI layer.
            // Sim just advances time.
            world.time.advance();
            PhaseResult::Midday
        }
        Phase::Evening => {
            // Resolve returning missions
            let mission_results = missions::resolve_returning_missions(world);

            world.time.advance();

            PhaseResult::Evening { mission_results }
        }
        Phase::Night => {
            // Expire old events
            events::expire_events(world);

            // TODO: spoilage, relic degradation, payroll deduction

            world.time.advance();
            PhaseResult::Night
        }
    }
}
