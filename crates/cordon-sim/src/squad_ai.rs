//! Squad AI: vision-shared engagement, formation positioning, goal
//! transitions, and squad lifecycle.
//!
//! Squads are Bevy entities. NPC members carry a [`SquadMembership`]
//! back-pointer. Hot-path systems iterate squads + members via ECS
//! queries — there is no HashMap fallback.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::squad::Goal;
use cordon_data::gamedata::GameDataResource;

use crate::behavior::{AnomalyZone, CombatTarget, Dead, MovementSpeed, MovementTarget, Vision};
use crate::combat::{is_hostile, line_blocked, weapon_range};
use crate::components::{
    LoadoutComp, NpcMarker, SquadActivity, SquadFacing, SquadFaction, SquadFormation, SquadGoal,
    SquadLeader, SquadMarker, SquadMembers, SquadMembership, SquadWaypoints, Xp,
};
use crate::plugin::SimSet;
use crate::resources::SquadIdIndex;

const SQUAD_WALK_SPEED: f32 = 30.0;
const ENGAGE_WALK_SPEED: f32 = 38.0;
const PATROL_HOLD_SECS: f32 = 6.0;
const ARRIVED_DIST: f32 = 12.0;

pub struct SquadAiPlugin;

impl Plugin for SquadAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                cleanup_dead_squads.in_set(SimSet::Cleanup),
                drive_squad_goals.in_set(SimSet::Goals),
                update_squad_engagement.in_set(SimSet::Engagement),
                drive_squad_formation.in_set(SimSet::Formation),
            ),
        );
    }
}

struct NpcSnap {
    entity: Entity,
    squad: Entity,
    pos: Vec2,
    vision: f32,
}

type SpatialGrid = HashMap<(i32, i32), Vec<usize>>;

fn build_spatial_grid(snapshot: &[NpcSnap], cell_size: f32) -> SpatialGrid {
    let mut grid: SpatialGrid = HashMap::with_capacity(snapshot.len() / 4 + 1);
    for (i, snap) in snapshot.iter().enumerate() {
        let cell = (
            (snap.pos.x / cell_size).floor() as i32,
            (snap.pos.y / cell_size).floor() as i32,
        );
        grid.entry(cell).or_default().push(i);
    }
    grid
}

fn collect_nearby_cells(
    center: Vec2,
    radius: f32,
    grid: &SpatialGrid,
    cell_size: f32,
    out: &mut Vec<usize>,
) {
    let min_cx = ((center.x - radius) / cell_size).floor() as i32;
    let max_cx = ((center.x + radius) / cell_size).floor() as i32;
    let min_cy = ((center.y - radius) / cell_size).floor() as i32;
    let max_cy = ((center.y + radius) / cell_size).floor() as i32;
    for cy in min_cy..=max_cy {
        for cx in min_cx..=max_cx {
            if let Some(bucket) = grid.get(&(cx, cy)) {
                out.extend_from_slice(bucket);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_squad_engagement(
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    anomalies: Query<(&Transform, &AnomalyZone)>,
    members_q: Query<
        (Entity, &SquadMembership, &Vision, &Transform),
        (With<NpcMarker>, Without<Dead>),
    >,
    squads_q: Query<(Entity, &SquadFaction, &SquadLeader), With<SquadMarker>>,
    mut squad_state_q: Query<(Entity, &mut SquadActivity, &mut SquadFacing, &SquadLeader)>,
    mut combat_targets_q: Query<&mut CombatTarget, Without<Dead>>,
) {
    const SCAN_INTERVAL_SECS: f32 = 0.1;
    *throttle += time.delta_secs();
    if *throttle < SCAN_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let factions = &game_data.0.factions;
    let anomaly_disks: Vec<(Vec2, f32)> = anomalies
        .iter()
        .map(|(t, a)| (t.translation.truncate(), a.radius))
        .collect();

    let snapshot: Vec<NpcSnap> = members_q
        .iter()
        .map(|(entity, member, vision, transform)| NpcSnap {
            entity,
            squad: member.squad,
            pos: transform.translation.truncate(),
            vision: vision.radius,
        })
        .collect();

    let squad_faction: HashMap<Entity, &cordon_core::primitive::Id<cordon_core::entity::faction::Faction>> = squads_q
        .iter()
        .map(|(e, f, _)| (e, &f.0))
        .collect();
    let squad_leader: HashMap<Entity, Entity> = squads_q
        .iter()
        .map(|(e, _, leader)| (e, leader.0))
        .collect();

    const CELL_SIZE: f32 = 200.0;
    let grid = build_spatial_grid(&snapshot, CELL_SIZE);

    // Pass A: per-squad — pick the hostile squad in vision.
    let mut squad_hostile: HashMap<Entity, Entity> = HashMap::new();
    for (squad_entity, _, _) in squads_q.iter() {
        let our_faction = match squad_faction.get(&squad_entity) {
            Some(f) => *f,
            None => continue,
        };
        let members: Vec<&NpcSnap> =
            snapshot.iter().filter(|n| n.squad == squad_entity).collect();
        if members.is_empty() {
            continue;
        }

        let mut candidates: Vec<usize> = Vec::new();
        for m in &members {
            collect_nearby_cells(m.pos, m.vision, &grid, CELL_SIZE, &mut candidates);
        }
        candidates.sort_unstable();
        candidates.dedup();

        let mut chosen: Option<(Entity, f32)> = None;
        for cand_idx in candidates {
            let cand = &snapshot[cand_idx];
            if cand.squad == squad_entity {
                continue;
            }
            let cand_faction = match squad_faction.get(&cand.squad) {
                Some(f) => *f,
                None => continue,
            };
            if !is_hostile(our_faction, cand_faction, factions) {
                continue;
            }
            let mut visible_dist_sq: Option<f32> = None;
            for m in &members {
                let d_sq = m.pos.distance_squared(cand.pos);
                let v_sq = m.vision * m.vision;
                if d_sq > v_sq {
                    continue;
                }
                if line_blocked(m.pos, cand.pos, &anomaly_disks) {
                    continue;
                }
                visible_dist_sq = Some(visible_dist_sq.map_or(d_sq, |d| d.min(d_sq)));
            }
            let Some(dist_sq) = visible_dist_sq else { continue };

            if chosen.is_none_or(|(_, d)| dist_sq < d) {
                chosen = Some((cand.squad, dist_sq));
            }
        }

        if let Some((hostile_squad, _)) = chosen {
            squad_hostile.insert(squad_entity, hostile_squad);
        }
    }

    // Pass B: write per-squad activity + facing.
    for (squad_entity, mut activity, mut facing, leader) in squad_state_q.iter_mut() {
        match squad_hostile.get(&squad_entity) {
            Some(hostile) => {
                let same = matches!(*activity, SquadActivity::Engage { hostiles } if hostiles == *hostile);
                if !same {
                    *activity = SquadActivity::Engage { hostiles: *hostile };
                }
                let our_pos = snapshot.iter().find(|n| n.entity == leader.0).map(|n| n.pos);
                let hostile_leader = squad_leader.get(hostile).copied();
                let hostile_pos = hostile_leader
                    .and_then(|h| snapshot.iter().find(|n| n.entity == h))
                    .map(|n| n.pos);
                if let (Some(p), Some(t)) = (our_pos, hostile_pos) {
                    let dir = (t - p).normalize_or_zero();
                    if dir.length_squared() > 0.001 {
                        facing.0 = dir;
                    }
                }
            }
            None => {
                if matches!(*activity, SquadActivity::Engage { .. }) {
                    *activity = SquadActivity::Hold { duration_secs: 0.5 };
                }
            }
        }
    }

    // Pass C: assign per-member CombatTarget.
    let snapshot_by_squad: HashMap<Entity, Vec<&NpcSnap>> = {
        let mut m: HashMap<Entity, Vec<&NpcSnap>> = HashMap::new();
        for n in &snapshot {
            m.entry(n.squad).or_default().push(n);
        }
        m
    };
    let activity_by_squad: HashMap<Entity, SquadActivity> = squad_state_q
        .iter()
        .map(|(e, a, _, _)| (e, (*a).clone()))
        .collect();

    for (entity, member, _, _) in members_q.iter() {
        let snap = match snapshot.iter().find(|n| n.entity == entity) {
            Some(s) => s,
            None => continue,
        };
        let activity = activity_by_squad.get(&member.squad);
        let Some(SquadActivity::Engage { hostiles }) = activity else {
            if let Ok(mut ct) = combat_targets_q.get_mut(entity)
                && ct.0.is_some()
            {
                ct.0 = None;
            }
            continue;
        };

        let Some(hostile_members) = snapshot_by_squad.get(&hostiles) else {
            if let Ok(mut ct) = combat_targets_q.get_mut(entity) {
                ct.0 = None;
            }
            continue;
        };
        let mut best: Option<(Entity, f32)> = None;
        for enemy in hostile_members {
            if line_blocked(snap.pos, enemy.pos, &anomaly_disks) {
                continue;
            }
            let dist_sq = snap.pos.distance_squared(enemy.pos);
            if best.is_none_or(|(_, d)| dist_sq < d) {
                best = Some((enemy.entity, dist_sq));
            }
        }
        if let Ok(mut ct) = combat_targets_q.get_mut(entity) {
            match best {
                Some((target, _)) => {
                    if ct.0 != Some(target) {
                        ct.0 = Some(target);
                    }
                }
                None => {
                    if ct.0.is_some() {
                        ct.0 = None;
                    }
                }
            }
        }
    }
}

fn drive_squad_goals(
    time: Res<Time>,
    mut squads_q: Query<(&SquadGoal, &mut SquadActivity, &mut SquadWaypoints)>,
) {
    let dt = time.delta_secs();
    for (goal, mut activity, mut waypoints) in &mut squads_q {
        if let SquadActivity::Hold { duration_secs } = &mut *activity {
            *duration_secs -= dt;
            if *duration_secs <= 0.0 {
                *activity = next_activity_for_goal(&goal.0, &mut waypoints);
            }
        }
    }
}

fn next_activity_for_goal(goal: &Goal, waypoints: &mut SquadWaypoints) -> SquadActivity {
    match goal {
        Goal::Idle => SquadActivity::Hold {
            duration_secs: 4.0,
        },
        Goal::Patrol { .. } | Goal::Scavenge { .. } => {
            if waypoints.points.is_empty() {
                SquadActivity::Hold {
                    duration_secs: 4.0,
                }
            } else {
                let idx = (waypoints.next as usize) % waypoints.points.len();
                let target = waypoints.points[idx];
                waypoints.next = ((idx + 1) % waypoints.points.len()) as u8;
                SquadActivity::Move { target }
            }
        }
        Goal::Protect { .. } => SquadActivity::Hold {
            duration_secs: 0.5,
        },
        _ => SquadActivity::Hold {
            duration_secs: 4.0,
        },
    }
}

#[allow(clippy::type_complexity)]
fn drive_squad_formation(
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
        (SquadActivity, Vec2, cordon_core::entity::squad::Formation, usize),
    > = squad_state_q
        .iter()
        .map(|(e, _, _, members, formation, activity, facing)| {
            (e, (activity.clone(), facing.0, formation.0, members.0.len()))
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

/// Squad lifecycle: drop dead members, promote new leaders, despawn
/// fully-dead squads. Throttled to 1 Hz.
fn cleanup_dead_squads(
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut commands: Commands,
    alive_q: Query<&Xp, (With<NpcMarker>, Without<Dead>)>,
    mut squads_q: Query<(Entity, &mut SquadMembers, &mut SquadLeader)>,
) {
    const CLEANUP_INTERVAL_SECS: f32 = 1.0;
    *throttle += time.delta_secs();
    if *throttle < CLEANUP_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    for (squad_entity, mut members, mut leader) in &mut squads_q {
        members.0.retain(|m| alive_q.get(*m).is_ok());
        if members.0.is_empty() {
            commands.entity(squad_entity).despawn();
            continue;
        }
        if alive_q.get(leader.0).is_err() {
            let new_leader = members
                .0
                .iter()
                .filter_map(|m| alive_q.get(*m).ok().map(|xp| (*m, xp.rank())))
                .max_by_key(|(_, r)| *r)
                .map(|(m, _)| m);
            if let Some(new) = new_leader {
                leader.0 = new;
            } else {
                commands.entity(squad_entity).despawn();
            }
        }
    }
}
