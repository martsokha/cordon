use std::collections::HashMap;

use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NamePool;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::Id;
use cordon_core::world::event::EventDef;
use cordon_core::world::mission::MissionResult;

use crate::simulation::npcs::{LoadoutContext, NpcGenerator};
use crate::simulation::{events, factions, missions, npcs};
use crate::state::world::World;

/// Results from advancing the day, for the game layer to consume.
pub struct DayResult {
    pub visitors: Vec<Npc>,
    pub events_started: usize,
    pub mission_results: Vec<MissionResult>,
}

/// Advance the world by one day. Returns what happened.
pub fn advance_day(
    world: &mut World,
    event_defs: &[EventDef],
    npc_gen: &impl NpcGenerator,
    name_pools: &HashMap<Id<Faction>, NamePool>,
    fallback_pool: &NamePool,
    loadout_ctx: &LoadoutContext<'_>,
) -> DayResult {
    let event_count_before = world.active_events.len();
    events::roll_daily_events(world, event_defs);
    let events_started = world.active_events.len() - event_count_before;

    let visitors =
        npcs::spawn_daily_visitors(world, npc_gen, name_pools, fallback_pool, loadout_ctx);
    factions::tick_factions(world);

    let mission_results = missions::resolve_missions(world);
    events::expire_events(world);

    world.time.advance_hours(12);

    DayResult {
        visitors,
        events_started,
        mission_results,
    }
}
