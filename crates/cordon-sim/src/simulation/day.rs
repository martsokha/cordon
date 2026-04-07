use cordon_core::world::event::EventDef;
use cordon_core::world::mission::MissionResult;

use crate::simulation::{events, factions, missions};
use crate::state::world::World;

/// Results from advancing the day, for the game layer to consume.
///
/// Population top-up is no longer part of `advance_day` — it's a
/// separate Bevy system in [`crate::spawn`] that runs on its own
/// schedule and reads the live ECS query for the current count.
pub struct DayResult {
    pub events_started: usize,
    pub mission_results: Vec<MissionResult>,
}

/// Advance the world by one day. Returns what happened.
pub fn advance_day(world: &mut World, event_defs: &[EventDef]) -> DayResult {
    let event_count_before = world.active_events.len();
    events::roll_daily_events(world, event_defs);
    let events_started = world.active_events.len() - event_count_before;

    factions::tick_factions(world);

    let mission_results = missions::resolve_missions(world);
    events::expire_events(world);

    world.time.advance_hours(12);

    DayResult {
        events_started,
        mission_results,
    }
}
