//! Vision-shared engagement scan.
//!
//! Throttled to ~10Hz, this system is the squad-AI brain: each squad
//! looks through all of its alive members' vision cones, picks the
//! nearest hostile squad anyone can see, and writes both the squad-
//! level [`SquadActivity::Engage`] and the per-member [`CombatTarget`]
//! that the firing system will read.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use super::scan::{NpcSnap, build_spatial_grid, collect_nearby_cells};
use crate::behavior::{AnomalyZone, CombatTarget, Dead, Vision};
use crate::combat::{is_hostile, line_blocked};
use crate::components::{
    NpcMarker, SquadActivity, SquadFacing, SquadFaction, SquadLeader, SquadMarker, SquadMembership,
};

pub(super) fn update_squad_engagement(
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

    let squad_faction: HashMap<
        Entity,
        &cordon_core::primitive::Id<cordon_core::entity::faction::Faction>,
    > = squads_q.iter().map(|(e, f, _)| (e, &f.0)).collect();
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
        let members: Vec<&NpcSnap> = snapshot
            .iter()
            .filter(|n| n.squad == squad_entity)
            .collect();
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
            let Some(dist_sq) = visible_dist_sq else {
                continue;
            };

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
                let same =
                    matches!(*activity, SquadActivity::Engage { hostiles } if hostiles == *hostile);
                if !same {
                    *activity = SquadActivity::Engage { hostiles: *hostile };
                }
                let our_pos = snapshot
                    .iter()
                    .find(|n| n.entity == leader.0)
                    .map(|n| n.pos);
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

        let Some(hostile_members) = snapshot_by_squad.get(hostiles) else {
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
