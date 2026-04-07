//! Day-rollover orchestrator (pre-ECS, currently called once at world
//! init). This will become an event-driven system fired on the
//! `DayRolled` event once the `World` resource is dissolved.

use cordon_core::world::event::EventDef;

use crate::world::events;
use crate::world::factions;
use crate::world::state::World;

/// Results from advancing the day, for the game layer to consume.
pub struct DayResult {
    pub events_started: usize,
}

/// Advance the world by one day. Returns what happened.
pub fn advance_day(world: &mut World, event_defs: &[EventDef]) -> DayResult {
    let event_count_before = world.active_events.len();
    events::roll_daily_events(world, event_defs);
    let events_started = world.active_events.len() - event_count_before;

    factions::tick_factions(world);
    events::expire_events(world);

    world.time.advance_hours(12);

    DayResult { events_started }
}
