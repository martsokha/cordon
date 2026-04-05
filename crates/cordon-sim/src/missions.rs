use cordon_core::economy::mission::{ActiveMission, MissionOutcome, MissionPlan, MissionResult};
use cordon_core::entity::npc::Perk;
use cordon_core::world::time::Day;
use rand::Rng;

use crate::world::World;

pub fn dispatch_mission(world: &mut World, plan: MissionPlan) -> Result<(), &'static str> {
    let runner = world
        .npcs
        .get(&plan.runner_id)
        .ok_or("runner not found")?;

    if !runner.is_employed() {
        return Err("NPC is not employed");
    }

    let required_radio = plan.destination.radio_level_required();
    if world.bunker.radio_level < required_radio {
        return Err("radio level too low for this sector");
    }

    let travel_days = plan.destination.travel_days();
    let return_day = Day(world.time.day.0 + travel_days);

    let mission = ActiveMission {
        plan,
        day_dispatched: world.time.day,
        return_day,
    };

    world.active_missions.push(mission);
    Ok(())
}

pub fn resolve_returning_missions(world: &mut World) -> Vec<MissionResult> {
    let current_day = world.time.day;
    let mut results = Vec::new();

    let (returning, still_out): (Vec<_>, Vec<_>) = world
        .active_missions
        .drain(..)
        .partition(|m| m.return_day <= current_day);

    world.active_missions = still_out;

    for mission in returning {
        let sector = world.sectors.get(&mission.plan.destination);
        let runner = world.npcs.get(&mission.plan.runner_id);

        let (danger, has_hard_to_kill, has_pathfinder, has_scavengers_eye, has_coward) =
            match (sector, runner) {
                (Some(s), Some(r)) => (
                    s.effective_danger(),
                    r.has_perk(Perk::HardToKill),
                    r.has_perk(Perk::Pathfinder),
                    r.has_perk(Perk::ScavengersEye),
                    r.has_perk(Perk::Coward),
                ),
                _ => (0.5, false, false, false, false),
            };

        let outcome = roll_outcome(danger, has_hard_to_kill, has_pathfinder, has_coward, &mut world.rng);

        let mut perks_revealed = Vec::new();

        // Reveal perks based on outcome
        if matches!(outcome, MissionOutcome::RunnerLost) && has_hard_to_kill {
            // They survived despite being "lost" — reveal HardToKill and upgrade to Failure
        }
        if matches!(outcome, MissionOutcome::Jackpot) && has_scavengers_eye {
            perks_revealed.push(Perk::ScavengersEye);
        }
        if matches!(outcome, MissionOutcome::Failure) && has_coward {
            perks_revealed.push(Perk::Coward);
        }
        if matches!(outcome, MissionOutcome::Success) && has_pathfinder {
            // Fast return could reveal Pathfinder
            if world.rng.gen_bool(0.3) {
                perks_revealed.push(Perk::Pathfinder);
            }
        }

        // Reveal perks on the NPC
        if let Some(runner) = world.npcs.get_mut(&mission.plan.runner_id) {
            for &perk in &perks_revealed {
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
    // Base probabilities from design doc, shifted by danger
    let mut p_success = 0.6 - danger * 0.4;
    let mut p_partial = 0.2;
    let mut p_failure = 0.1 + danger * 0.2;
    let mut p_lost = 0.05 + danger * 0.15;
    let mut p_jackpot = 0.05;

    // Perk modifiers
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
        p_lost *= 0.5; // cowards flee, they don't die
    }

    // Normalize
    let total = p_success + p_partial + p_failure + p_lost + p_jackpot;
    p_success /= total;
    p_partial /= total;
    p_failure /= total;
    let _ = p_lost / total; // used implicitly as remainder
    p_jackpot /= total;

    let roll: f32 = rng.r#gen();
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
