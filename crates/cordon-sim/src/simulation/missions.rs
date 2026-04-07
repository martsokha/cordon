use cordon_core::primitive::{Day, Location};
use cordon_core::world::mission::{ActiveMission, MissionPlan, MissionResult};

use crate::state::world::World;

/// Dispatch a runner on a mission. Validates prerequisites and computes
/// the return day based on sector distance and runner perks.
///
/// TODO: rewrite as a Bevy system that reads runner perks from ECS
/// components. The function-on-`World` form is a holdover from the
/// pre-ECS sim.
pub fn dispatch_mission(world: &mut World, plan: MissionPlan) -> Result<(), &'static str> {
    let travel_days = 1_u32;
    let return_day = Day::new(world.time.day.value() + travel_days);
    let mission = ActiveMission {
        plan,
        day_dispatched: world.time.day,
        return_day,
        current_location: Location::ORIGIN,
    };
    world.active_missions.push(mission);
    Ok(())
}

/// Resolve all missions whose runners have returned (return_day <= today).
///
/// TODO: rewrite as a Bevy system that looks runners up by entity and
/// reads their perks/loadout from ECS. Currently a stub that returns
/// empty results so the day cycle keeps working.
pub fn resolve_missions(world: &mut World) -> Vec<MissionResult> {
    let current_day = world.time.day;
    let (_returning, still_out): (Vec<_>, Vec<_>) = world
        .active_missions
        .drain(..)
        .partition(|m| m.return_day <= current_day);
    world.active_missions = still_out;
    Vec::new()
}

