//! Formation positioning and movement targeting.
//!
//! Throttled to ~10Hz. For each squad this:
//!
//! 1. Snapshots the leader position.
//! 2. Resolves `Goal::Protect` to a Move target via `SquadIdIndex`.
//! 3. Flips arrived `Move` activities to `Hold` and updates facing.
//! 4. Per member, writes `MovementTarget`/`MovementSpeed` to either
//!    a combat target (if engaging and out of weapon range) or to the
//!    formation slot in the squad's local frame.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::squad::Goal;
use cordon_data::gamedata::GameDataResource;

use super::{ARRIVED_DIST, ENGAGE_WALK_SPEED, PATROL_HOLD_SECS, SQUAD_WALK_SPEED};
use crate::behavior::{CombatTarget, Dead, MovementSpeed, MovementTarget};
use crate::combat::weapon_range;
use crate::components::{
    LoadoutComp, NpcMarker, SquadActivity, SquadFacing, SquadFormation, SquadGoal, SquadLeader,
    SquadMarker, SquadMembers, SquadMembership,
};
use crate::resources::SquadIdIndex;

pub(super) fn drive_squad_formation(
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    squad_index: Res<SquadIdIndex>,
    mut throttle: Local<f32>,
    mut squad_state_q: Query<(
        Entity,
        &SquadGoal,
        &SquadLeader,
        &SquadMembers,
        &SquadFormation,
        &mut SquadActivity,
        &mut SquadFacing,
    )>,
    other_leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    leaders_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut members_q: Query<
        (
            &SquadMembership,
            &Transform,
            &CombatTarget,
            &LoadoutComp,
            &mut MovementTarget,
            &mut MovementSpeed,
        ),
        Without<Dead>,
    >,
    targets_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
) {
    const FORMATION_INTERVAL_SECS: f32 = 0.1;
    *throttle += time.delta_secs();
    if *throttle < FORMATION_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let items = &game_data.0.items;

    let mut squad_leader_pos: HashMap<Entity, Vec2> = HashMap::new();
    for (squad_entity, _, leader, _, _, _, _) in squad_state_q.iter() {
        if let Ok(t) = leaders_q.get(leader.0) {
            squad_leader_pos.insert(squad_entity, t.translation.truncate());
        }
    }

    // Pass A: per-squad — handle Protect, arrival flip, facing.
    for (squad_entity, goal, _, _, _, mut activity, mut facing) in squad_state_q.iter_mut() {
        let Some(p) = squad_leader_pos.get(&squad_entity).copied() else {
            continue;
        };

        if let Goal::Protect { other } = &goal.0
            && !matches!(*activity, SquadActivity::Engage { .. })
            && let Some(other_entity) = squad_index.0.get(other).copied()
            && let Ok(other_leader) = other_leaders_q.get(other_entity)
            && let Ok(other_t) = leaders_q.get(other_leader.0)
        {
            let target = other_t.translation.truncate();
            const PROTECT_FOLLOW_DIST: f32 = 40.0;
            if p.distance(target) > PROTECT_FOLLOW_DIST {
                *activity = SquadActivity::Move { target };
            } else if matches!(*activity, SquadActivity::Move { .. }) {
                *activity = SquadActivity::Hold { duration_secs: 0.5 };
            }
        }

        if let SquadActivity::Move { target } = *activity
            && p.distance(target) < ARRIVED_DIST
        {
            *activity = SquadActivity::Hold {
                duration_secs: PATROL_HOLD_SECS,
            };
        }

        let new_facing = match *activity {
            SquadActivity::Move { target } => (target - p).normalize_or_zero(),
            _ => facing.0,
        };
        if new_facing.length_squared() > 0.001 {
            facing.0 = new_facing;
        } else if facing.0 == Vec2::ZERO {
            facing.0 = Vec2::Y;
        }
    }

    let squad_info: HashMap<
        Entity,
        (
            SquadActivity,
            Vec2,
            cordon_core::entity::squad::Formation,
            usize,
        ),
    > = squad_state_q
        .iter()
        .map(|(e, _, _, members, formation, activity, facing)| {
            (
                e,
                (activity.clone(), facing.0, formation.0, members.0.len()),
            )
        })
        .collect();

    for (member, transform, combat_target, loadout, mut move_target, mut speed) in &mut members_q {
        let Some((activity, facing_v, formation, member_count)) =
            squad_info.get(&member.squad).cloned()
        else {
            continue;
        };
        let pos = transform.translation.truncate();

        if matches!(activity, SquadActivity::Engage { .. }) {
            if let Some(target_entity) = combat_target.0
                && let Ok(target_t) = targets_q.get(target_entity)
            {
                let target_pos = target_t.translation.truncate();
                let dist = pos.distance(target_pos);
                let range = weapon_range(items, &loadout.0);
                if range > 0.0 && dist <= range {
                    if move_target.0.is_some() {
                        move_target.0 = None;
                    }
                } else {
                    set_movement_target(
                        &mut move_target,
                        &mut speed,
                        target_pos,
                        ENGAGE_WALK_SPEED,
                    );
                }
            } else if move_target.0.is_some() {
                move_target.0 = None;
            }
            continue;
        }

        let Some(leader_p) = squad_leader_pos.get(&member.squad).copied() else {
            continue;
        };
        let facing = facing_v.normalize_or_zero();
        if facing.length_squared() < 0.001 {
            continue;
        }
        let centroid = match activity {
            SquadActivity::Hold { .. } => leader_p,
            SquadActivity::Move { target } => target,
            SquadActivity::Engage { .. } => leader_p,
        };
        let offsets = formation.slot_offsets(member_count);
        let slot = (member.slot as usize).min(offsets.len().saturating_sub(1));
        let local = Vec2::new(offsets[slot][0], offsets[slot][1]);
        let perp = Vec2::new(-facing.y, facing.x);
        let world_offset = perp * local.x + facing * local.y;
        let target = centroid + world_offset;

        let dist = pos.distance(target);
        if dist > ARRIVED_DIST {
            set_movement_target(&mut move_target, &mut speed, target, SQUAD_WALK_SPEED);
        } else if move_target.0.is_some() {
            move_target.0 = None;
        }
    }
}

fn set_movement_target(
    move_target: &mut MovementTarget,
    speed: &mut MovementSpeed,
    new_target: Vec2,
    new_speed: f32,
) {
    let same = move_target
        .0
        .map(|t| t.distance(new_target) < 1.0)
        .unwrap_or(false);
    if !same {
        move_target.0 = Some(new_target);
    }
    if (speed.0 - new_speed).abs() > 0.5 {
        speed.0 = new_speed;
    }
}
