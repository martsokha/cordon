//! Vision-shared engagement scan.
//!
//! Throttled to ~10Hz, this system is the squad-AI brain: each
//! squad looks through all of its alive members' vision cones,
//! picks the nearest hostile squad anyone can see, and writes both
//! the squad-level [`EngagementTarget`] and the per-member
//! [`CombatTarget`] that the firing system will read.
//!
//! The scanner runs unconditionally — engagement is never suppressed.
//! Behavior trees may branch on `EngagementTarget` but do not gate
//! it.
//!
//! # Three-pass structure
//!
//! Bevy's query rules forbid overlapping mut borrows on the same
//! archetype, so this system splits the work into three passes
//! with staged results in between:
//!
//! 1. **Pass A** (per squad) — walk each squad's members, gather
//!    candidate hostiles from the spatial grid, run LOS checks,
//!    pick the nearest visible hostile squad. Staged in a local
//!    `HashMap<our_squad, hostile_squad>`.
//! 2. **Pass B** (per squad, mutable) — write the staged hostile
//!    into [`EngagementTarget`] and update facing toward the
//!    hostile leader.
//! 3. **Pass C** (per member) — pick each engaged member's
//!    closest hostile member (preferring LOS) and write
//!    [`CombatTarget`].

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

use super::constants::{ENGAGEMENT_CELL_SIZE, SCAN_INTERVAL_SECS};
use super::formation::SquadFacing;
use super::identity::{SquadLeader, SquadMarker, SquadMembership};
use super::intent::EngagementTarget;
use super::scan::{NpcSnap, SpatialGrid};
use crate::behavior::combat::components::CombatTarget;
use crate::behavior::combat::helpers::{is_hostile, line_blocked};
use crate::behavior::death::components::Dead;
use crate::behavior::vision::components::{AnomalyZone, Vision};
use crate::entity::npc::{Essential, FactionId, NpcMarker};

pub(super) fn update_squad_engagement(
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut grid: Local<SpatialGrid>,
    anomalies: Query<(&Transform, &AnomalyZone)>,
    members_q: Query<
        (Entity, &SquadMembership, &Vision, &Transform),
        (With<NpcMarker>, Without<Dead>, Without<Essential>),
    >,
    squads_q: Query<(Entity, &FactionId, &SquadLeader), With<SquadMarker>>,
    mut squad_state_q: Query<(
        Entity,
        &mut EngagementTarget,
        &mut SquadFacing,
        &SquadLeader,
    )>,
    mut combat_targets_q: Query<&mut CombatTarget, (Without<Dead>, Without<Essential>)>,
) {
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

    let squad_faction: HashMap<Entity, &Id<Faction>> =
        squads_q.iter().map(|(e, f, _)| (e, &f.0)).collect();
    let squad_leader: HashMap<Entity, Entity> = squads_q
        .iter()
        .map(|(e, _, leader)| (e, leader.0))
        .collect();

    grid.rebuild(&snapshot, ENGAGEMENT_CELL_SIZE);

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
            grid.collect_nearby(m.pos, m.vision, ENGAGEMENT_CELL_SIZE, &mut candidates);
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

    // Pass B: write per-squad engagement target + facing.
    for (squad_entity, mut engagement, mut facing, leader) in squad_state_q.iter_mut() {
        match squad_hostile.get(&squad_entity) {
            Some(hostile) => {
                if engagement.0 != Some(*hostile) {
                    engagement.0 = Some(*hostile);
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
                if engagement.0.is_some() {
                    engagement.0 = None;
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
    let engagement_by_squad: HashMap<Entity, Option<Entity>> = squad_state_q
        .iter()
        .map(|(e, et, _, _)| (e, et.0))
        .collect();

    for (entity, member, _, _) in members_q.iter() {
        let snap = match snapshot.iter().find(|n| n.entity == entity) {
            Some(s) => s,
            None => continue,
        };
        // Squad isn't engaged: drop any stale per-member target.
        let Some(Some(hostiles)) = engagement_by_squad.get(&member.squad).copied() else {
            if let Ok(mut ct) = combat_targets_q.get_mut(entity)
                && ct.0.is_some()
            {
                ct.0 = None;
            }
            continue;
        };
        let hostiles = &hostiles;

        // Hostile squad has no alive members in snapshot — they all
        // died this tick. Drop the per-member target; the next scan
        // will pick a fresh hostile or clear engagement entirely.
        let Some(hostile_members) = snapshot_by_squad.get(hostiles) else {
            if let Ok(mut ct) = combat_targets_q.get_mut(entity) {
                ct.0 = None;
            }
            continue;
        };

        // Pick the closest hostile member with clear LOS from this
        // member's position. If none have LOS, fall back to the
        // closest hostile member regardless — formation will move
        // this member toward them so they can regain LOS, instead
        // of leaving them stranded with no target.
        let mut best_los: Option<(Entity, f32)> = None;
        let mut best_any: Option<(Entity, f32)> = None;
        for enemy in hostile_members {
            let dist_sq = snap.pos.distance_squared(enemy.pos);
            if best_any.is_none_or(|(_, d)| dist_sq < d) {
                best_any = Some((enemy.entity, dist_sq));
            }
            if line_blocked(snap.pos, enemy.pos, &anomaly_disks) {
                continue;
            }
            if best_los.is_none_or(|(_, d)| dist_sq < d) {
                best_los = Some((enemy.entity, dist_sq));
            }
        }
        let chosen_target = best_los.or(best_any).map(|(e, _)| e);
        if let Ok(mut ct) = combat_targets_q.get_mut(entity)
            && ct.0 != chosen_target
        {
            ct.0 = chosen_target;
        }
    }
}
