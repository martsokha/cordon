use cordon_core::entity::perk::Perk;
use cordon_core::primitive::{Day, Id, Location};
use cordon_core::world::mission::{ActiveMission, MissionOutcome, MissionPlan, MissionResult};
use rand::{Rng, RngExt};

use crate::state::world::World;

/// Well-known perk IDs (must match config).
const PERK_HARD_TO_KILL: &str = "hard_to_kill";
const PERK_PATHFINDER: &str = "pathfinder";
const PERK_SCAVENGERS_EYE: &str = "scavengers_eye";
const PERK_COWARD: &str = "coward";

/// Dispatch a runner on a mission. Validates prerequisites.
/// Dispatch a runner on a mission. Validates prerequisites and computes
/// the return day based on sector distance and runner perks.
pub fn dispatch_mission(world: &mut World, plan: MissionPlan) -> Result<(), &'static str> {
    let runner = world.npcs.get(&plan.runner_id).ok_or("runner not found")?;

    if !runner.is_employed() {
        return Err("NPC is not employed");
    }

    // TODO: check radio level against sector config
    // TODO: compute travel time from sector distance, events, runner perks
    let travel_days = 1_u32; // placeholder

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
pub fn resolve_missions(world: &mut World) -> Vec<MissionResult> {
    let current_day = world.time.day;
    let mut results = Vec::new();

    let (returning, still_out): (Vec<_>, Vec<_>) = world
        .active_missions
        .drain(..)
        .partition(|m| m.return_day <= current_day);

    world.active_missions = still_out;

    let htk_id = Id::<Perk>::new(PERK_HARD_TO_KILL);
    let pf_id = Id::<Perk>::new(PERK_PATHFINDER);
    let se_id = Id::<Perk>::new(PERK_SCAVENGERS_EYE);
    let cow_id = Id::<Perk>::new(PERK_COWARD);

    for mission in returning {
        let runner = world.npcs.get(&mission.plan.runner_id);

        let (has_hard_to_kill, has_pathfinder, has_scavengers_eye, has_coward) = match runner {
            Some(r) => (
                r.has_perk(&htk_id),
                r.has_perk(&pf_id),
                r.has_perk(&se_id),
                r.has_perk(&cow_id),
            ),
            None => (false, false, false, false),
        };

        // TODO: get base danger from sector config
        let danger = 0.5_f32;
        let outcome = roll_outcome(
            danger,
            has_hard_to_kill,
            has_pathfinder,
            has_coward,
            &mut world.rng.missions,
        );

        let mut perks_revealed = Vec::new();

        if matches!(outcome, MissionOutcome::Jackpot) && has_scavengers_eye {
            perks_revealed.push(se_id.clone());
        }
        if matches!(outcome, MissionOutcome::Failure) && has_coward {
            perks_revealed.push(cow_id.clone());
        }
        if matches!(outcome, MissionOutcome::Success)
            && has_pathfinder
            && world.rng.missions.random_bool(0.3)
        {
            perks_revealed.push(pf_id.clone());
        }

        if let Some(runner) = world.npcs.get_mut(&mission.plan.runner_id) {
            for perk in &perks_revealed {
                runner.reveal_perk(perk);
            }
        }

        results.push(MissionResult {
            mission_id: mission.plan.id,
            outcome,
            loot: Vec::new(), // TODO: roll loot from loot tables
            runner_condition_delta: match outcome {
                MissionOutcome::Success | MissionOutcome::Jackpot => 0.0,
                MissionOutcome::PartialSuccess => -0.1,
                MissionOutcome::Failure => -0.3,
                MissionOutcome::RunnerLost => -1.0,
            },
            gear_condition_delta: match outcome {
                MissionOutcome::Success | MissionOutcome::Jackpot => -0.02,
                MissionOutcome::PartialSuccess => -0.05,
                MissionOutcome::Failure => -0.15,
                MissionOutcome::RunnerLost => -1.0,
            },
            perks_revealed,
        });
    }

    results
}

fn roll_outcome(
    danger: f32,
    hard_to_kill: bool,
    pathfinder: bool,
    coward: bool,
    rng: &mut impl Rng,
) -> MissionOutcome {
    let mut p_success = 0.6 - danger * 0.4;
    let mut p_partial = 0.2;
    let mut p_failure = 0.1 + danger * 0.2;
    let mut p_lost = 0.05 + danger * 0.15;
    let p_jackpot = 0.05;

    if hard_to_kill {
        p_lost *= 0.3;
        p_failure += p_lost * 0.5;
    }
    if pathfinder {
        p_success += 0.1;
        p_failure -= 0.05;
    }
    if coward {
        p_failure += 0.15;
        p_success -= 0.1;
        p_lost *= 0.5;
    }

    let total = p_success + p_partial + p_failure + p_lost + p_jackpot;
    p_success /= total;
    p_partial /= total;
    p_failure /= total;
    let p_jackpot = p_jackpot / total;

    let roll: f32 = rng.random::<f32>();
    let mut cumulative = 0.0;

    cumulative += p_jackpot;
    if roll < cumulative {
        return MissionOutcome::Jackpot;
    }
    cumulative += p_success;
    if roll < cumulative {
        return MissionOutcome::Success;
    }
    cumulative += p_partial;
    if roll < cumulative {
        return MissionOutcome::PartialSuccess;
    }
    cumulative += p_failure;
    if roll < cumulative {
        return MissionOutcome::Failure;
    }

    MissionOutcome::RunnerLost
}
