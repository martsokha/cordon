//! Squad ECS layer: per-NPC SquadMember component, formation
//! positioning, goal-driven activity, vision sharing, focus fire, and
//! squad lifecycle (leader promotion + cleanup).
//!
//! The squad data itself lives in [`cordon_sim::state::world::World::squads`].
//! Per-NPC entities carry a [`SquadMember`] back-pointer plus the
//! shared movement/combat target components.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::{Goal, Squad};
use cordon_core::primitive::Uid;
use cordon_data::gamedata::GameDataResource;

use super::AiSet;
use super::behavior::{CombatTarget, MovementSpeed, MovementTarget};
use super::combat::{AnomalyZone, Vision, is_hostile, line_blocked, weapon_range};
use super::death::Dead;
use crate::PlayingState;
use crate::laptop::NpcDot;
use crate::world::SimWorld;

/// Back-pointer from an NPC entity to its [`Squad`].
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMember {
    pub squad: Uid<Squad>,
    /// Formation slot index (0 = leader, 1..=4 = followers).
    pub slot: u8,
}

/// Short-term squad state: holding, moving as a group, or engaged.
/// Long-term reasons live on [`Squad::goal`].
#[derive(Debug, Clone)]
pub enum Activity {
    /// Standing still, waiting for the next directive from the goal.
    Hold { duration_secs: f32 },
    /// Moving the whole squad to a position in the leader-relative formation.
    Move { target: Vec2 },
    /// Focus fire on a hostile squad. Members independently pick their
    /// own nearest enemy from this squad to engage.
    Engage { hostiles: Uid<Squad> },
}

/// How fast a squad walks in non-combat movement.
const SQUAD_WALK_SPEED: f32 = 30.0;
/// Slightly faster speed when closing into firing range.
const ENGAGE_WALK_SPEED: f32 = 38.0;
/// How long a squad holds at a patrol waypoint before moving on.
const PATROL_HOLD_SECS: f32 = 6.0;
/// Distance below which a squad considers a Move target reached.
const ARRIVED_DIST: f32 = 12.0;

/// Plugin registering the squad systems. Each system declares its
/// place in the shared [`super::AiSet`] schedule so cross-plugin order
/// is well defined.
pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadActivities>();
        app.add_systems(
            Update,
            (
                cleanup_dead_squads.in_set(AiSet::Cleanup),
                drive_squad_goals.in_set(AiSet::Goals),
                update_squad_engagement.in_set(AiSet::Engagement),
                drive_squad_formation.in_set(AiSet::Formation),
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Per-squad activity stored as a Bevy resource.
#[derive(Resource, Default)]
pub struct SquadActivities(pub HashMap<Uid<Squad>, Activity>);

// ====================================================================
// Engagement scanning (vision shared by squad).
// ====================================================================

/// One snapshot of an alive NPC's spatial state.
struct NpcSnap {
    uid: Uid<Npc>,
    squad: Uid<Squad>,
    pos: Vec2,
    vision: f32,
}

/// 2D spatial hash grid mapping cell coordinates to snapshot indices.
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

/// Each tick (throttled), scan for hostile-squad sightings via shared
/// squad vision. Updates the squad's [`Activity::Engage`] *and* writes
/// each member's [`CombatTarget`] to their nearest reachable enemy
/// from the hostile squad. When no hostile is in sight, drops back to
/// Hold and clears combat targets.
#[allow(clippy::type_complexity)]
fn update_squad_engagement(
    sim: Option<ResMut<SimWorld>>,
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut activities: ResMut<SquadActivities>,
    anomalies: Query<(&Transform, &AnomalyZone)>,
    members_q: Query<
        (&NpcDot, &SquadMember, &Vision, &Transform),
        Without<Dead>,
    >,
    mut targets_q: Query<(&NpcDot, &mut CombatTarget), Without<Dead>>,
) {
    // Throttle: scan ~10Hz instead of every frame. Combat firing
    // (which reads CombatTarget) still runs every frame so reactivity
    // is preserved.
    const SCAN_INTERVAL_SECS: f32 = 0.1;
    *throttle += time.delta_secs();
    if *throttle < SCAN_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let Some(mut sim) = sim else { return };
    let factions = &game_data.0.factions;

    let anomaly_disks: Vec<(Vec2, f32)> = anomalies
        .iter()
        .map(|(t, a)| (t.translation.truncate(), a.radius))
        .collect();

    // Snapshot every alive NPC.
    let mut snapshot: Vec<NpcSnap> = Vec::with_capacity(members_q.iter().count());
    for (dot, member, vision, transform) in &members_q {
        if !sim.0.npcs.contains_key(&dot.uid) {
            continue;
        }
        snapshot.push(NpcSnap {
            uid: dot.uid,
            squad: member.squad,
            pos: transform.translation.truncate(),
            vision: vision.radius,
        });
    }

    // Spatial grid for fast nearby-NPC lookups.
    const CELL_SIZE: f32 = 200.0;
    let grid = build_spatial_grid(&snapshot, CELL_SIZE);

    // Pass A: per-squad — pick the hostile squad in vision.
    let mut squad_hostile: HashMap<Uid<Squad>, Uid<Squad>> = HashMap::new();
    for (squad_uid, squad) in &sim.0.squads {
        let members: Vec<&NpcSnap> =
            snapshot.iter().filter(|n| n.squad == *squad_uid).collect();
        if members.is_empty() {
            continue;
        }

        // Collect candidate NPC indices from grid cells overlapping
        // any member's vision disc.
        let mut candidates: Vec<usize> = Vec::new();
        for m in &members {
            collect_nearby_cells(m.pos, m.vision, &grid, CELL_SIZE, &mut candidates);
        }
        candidates.sort_unstable();
        candidates.dedup();

        let mut chosen: Option<(Uid<Squad>, f32)> = None;
        for cand_idx in candidates {
            let cand = &snapshot[cand_idx];
            if cand.squad == *squad_uid {
                continue;
            }
            let Some(cand_squad) = sim.0.squads.get(&cand.squad) else {
                continue;
            };
            if !is_hostile(&squad.faction, &cand_squad.faction, factions) {
                continue;
            }
            // Visible to any of our members?
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
                visible_dist_sq =
                    Some(visible_dist_sq.map_or(d_sq, |d| d.min(d_sq)));
            }
            let Some(dist_sq) = visible_dist_sq else { continue };

            if chosen.is_none_or(|(_, d)| dist_sq < d) {
                chosen = Some((cand.squad, dist_sq));
            }
        }

        if let Some((hostile_squad_uid, _)) = chosen {
            squad_hostile.insert(*squad_uid, hostile_squad_uid);
        }
    }

    // Snapshot leader uids for the facing-update pass.
    let leader_by_squad: HashMap<Uid<Squad>, Uid<Npc>> = sim
        .0
        .squads
        .iter()
        .map(|(uid, sq)| (*uid, sq.leader))
        .collect();

    // Pass B: update per-squad activity + facing.
    for (squad_uid, squad) in sim.0.squads.iter_mut() {
        let activity = activities
            .0
            .entry(*squad_uid)
            .or_insert(Activity::Hold { duration_secs: 1.0 });
        match squad_hostile.get(squad_uid) {
            Some(hostile) => {
                let same = matches!(activity, Activity::Engage { hostiles } if hostiles == hostile);
                if !same {
                    *activity = Activity::Engage { hostiles: *hostile };
                }
                // Point this squad's facing toward the hostile leader.
                let our_leader_pos = snapshot
                    .iter()
                    .find(|n| n.uid == squad.leader)
                    .map(|n| n.pos);
                let hostile_leader_pos = leader_by_squad
                    .get(hostile)
                    .and_then(|h| snapshot.iter().find(|n| n.uid == *h))
                    .map(|n| n.pos);
                if let (Some(p), Some(t)) = (our_leader_pos, hostile_leader_pos) {
                    let dir = (t - p).normalize_or_zero();
                    if dir.length_squared() > 0.001 {
                        squad.facing = [dir.x, dir.y];
                    }
                }
            }
            None => {
                if matches!(activity, Activity::Engage { .. }) {
                    *activity = Activity::Hold { duration_secs: 0.5 };
                }
            }
        }
    }

    // Pass C: assign per-member CombatTarget for engaging squads,
    // clear it for everyone else.
    let snapshot_by_squad: HashMap<Uid<Squad>, Vec<&NpcSnap>> = {
        let mut m: HashMap<Uid<Squad>, Vec<&NpcSnap>> = HashMap::new();
        for n in &snapshot {
            m.entry(n.squad).or_default().push(n);
        }
        m
    };

    for (npc_dot, mut combat_target) in &mut targets_q {
        let snap = snapshot.iter().find(|n| n.uid == npc_dot.uid);
        let Some(snap) = snap else {
            // No snapshot entry means we couldn't read the NPC; clear
            // any stale target so combat doesn't keep firing.
            combat_target.0 = None;
            continue;
        };
        let activity = activities.0.get(&snap.squad);
        let Some(Activity::Engage { hostiles }) = activity else {
            // Not engaging — clear our target.
            if combat_target.0.is_some() {
                combat_target.0 = None;
            }
            continue;
        };
        // Find this NPC's nearest reachable enemy in the hostile squad.
        let Some(hostile_members) = snapshot_by_squad.get(hostiles) else {
            combat_target.0 = None;
            continue;
        };
        let mut best: Option<(Uid<Npc>, f32)> = None;
        for enemy in hostile_members {
            if line_blocked(snap.pos, enemy.pos, &anomaly_disks) {
                continue;
            }
            let dist_sq = snap.pos.distance_squared(enemy.pos);
            if best.is_none_or(|(_, d)| dist_sq < d) {
                best = Some((enemy.uid, dist_sq));
            }
        }
        match best {
            Some((target_uid, _)) => {
                if combat_target.0 != Some(target_uid) {
                    combat_target.0 = Some(target_uid);
                }
            }
            None => {
                if combat_target.0.is_some() {
                    combat_target.0 = None;
                }
            }
        }
    }
}

// ====================================================================
// Goal-driven activity transitions.
// ====================================================================

/// Hold timer expired → pick the next activity from the squad's goal.
fn drive_squad_goals(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    mut activities: ResMut<SquadActivities>,
) {
    let Some(mut sim) = sim else { return };
    let dt = time.delta_secs();

    for (squad_uid, squad) in sim.0.squads.iter_mut() {
        if squad.members.is_empty() {
            continue;
        }
        let activity = activities
            .0
            .entry(*squad_uid)
            .or_insert(Activity::Hold { duration_secs: 1.0 });

        if let Activity::Hold { duration_secs } = activity {
            *duration_secs -= dt;
            if *duration_secs <= 0.0 {
                *activity = next_activity_for_squad(squad);
            }
        }
    }
}

/// Compute the next activity for a squad based on its goal.
fn next_activity_for_squad(squad: &mut Squad) -> Activity {
    match &squad.goal {
        Goal::Idle => Activity::Hold {
            duration_secs: 4.0,
        },
        Goal::Patrol { .. } | Goal::Scavenge { .. } => {
            if squad.waypoints.is_empty() {
                Activity::Hold {
                    duration_secs: 4.0,
                }
            } else {
                let idx = (squad.next_waypoint as usize) % squad.waypoints.len();
                let wp = squad.waypoints[idx];
                squad.next_waypoint = ((idx + 1) % squad.waypoints.len()) as u8;
                Activity::Move {
                    target: Vec2::new(wp[0], wp[1]),
                }
            }
        }
        Goal::Protect { .. } => {
            // The formation system overrides this each tick to track
            // the protected squad's leader. A short Hold is fine while
            // we wait for that update.
            Activity::Hold {
                duration_secs: 0.5,
            }
        }
        _ => Activity::Hold {
            duration_secs: 4.0,
        },
    }
}

// ====================================================================
// Formation positioning — write MovementTarget for every member.
// ====================================================================

/// Update each member's `MovementTarget` to their formation slot
/// (relative to the leader + squad facing). Squads in `Activity::Move`
/// also get their facing updated and arrival flipped to Hold. Squads
/// in `Activity::Engage` have their members move toward their assigned
/// `CombatTarget` directly (or hold if in range).
///
/// Throttled to 10Hz: positions only need to refresh as fast as the
/// eye can perceive, and at large populations every-frame iteration
/// over ~1000 NPCs was a hot path.
#[allow(clippy::type_complexity)]
fn drive_squad_formation(
    sim: Option<ResMut<SimWorld>>,
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut activities: ResMut<SquadActivities>,
    mut members_q: Query<
        (
            &NpcDot,
            &SquadMember,
            &Transform,
            &CombatTarget,
            &mut MovementTarget,
            &mut MovementSpeed,
        ),
        Without<Dead>,
    >,
    leaders_q: Query<(&NpcDot, &Transform), Without<Dead>>,
) {
    const FORMATION_INTERVAL_SECS: f32 = 0.1;
    *throttle += time.delta_secs();
    if *throttle < FORMATION_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let Some(mut sim) = sim else { return };
    let items = &game_data.0.items;

    // Snapshot leader positions per squad uid.
    let leader_pos: HashMap<Uid<Npc>, Vec2> = leaders_q
        .iter()
        .map(|(dot, t)| (dot.uid, t.translation.truncate()))
        .collect();
    let mut squad_leader_pos: HashMap<Uid<Squad>, Vec2> = HashMap::new();
    for (squad_uid, squad) in &sim.0.squads {
        if let Some(p) = leader_pos.get(&squad.leader).copied() {
            squad_leader_pos.insert(*squad_uid, p);
        }
    }
    // Snapshot all NPC positions for combat-target lookups.
    let npc_pos: HashMap<Uid<Npc>, Vec2> = leader_pos.clone();

    // Pass A: per-squad — update facing, flip arrived Move → Hold,
    // override Activity for Goal::Protect.
    for (squad_uid, squad) in sim.0.squads.iter_mut() {
        let Some(p) = squad_leader_pos.get(squad_uid).copied() else {
            continue;
        };

        // Goal::Protect overrides activity each tick.
        if let Goal::Protect { other } = &squad.goal
            && let Some(other_pos) = squad_leader_pos.get(other).copied()
        {
            activities
                .0
                .insert(*squad_uid, Activity::Move { target: other_pos });
        }

        // Arrived Move → Hold.
        if let Some(activity) = activities.0.get_mut(squad_uid)
            && let Activity::Move { target } = activity
            && p.distance(*target) < ARRIVED_DIST
        {
            *activity = Activity::Hold {
                duration_secs: PATROL_HOLD_SECS,
            };
        }

        // Update facing.
        let new_facing = match activities.0.get(squad_uid) {
            Some(Activity::Move { target }) => (*target - p).normalize_or_zero(),
            _ => Vec2::new(squad.facing[0], squad.facing[1]),
        };
        if new_facing.length_squared() > 0.001 {
            squad.facing = [new_facing.x, new_facing.y];
        } else if squad.facing == [0.0, 0.0] {
            squad.facing = [0.0, 1.0];
        }
    }

    // Pass B: write each member's MovementTarget.
    for (npc_dot, member, transform, combat_target, mut move_target, mut speed) in
        &mut members_q
    {
        let Some(squad) = sim.0.squads.get(&member.squad) else {
            continue;
        };
        let Some(activity) = activities.0.get(&member.squad) else {
            continue;
        };
        let pos = transform.translation.truncate();

        // === Engaging? Move toward our personal combat target. ===
        if matches!(activity, Activity::Engage { .. }) {
            // If we have a target, walk toward it (combat will check
            // weapon range and stop us firing/move us into range).
            if let Some(target_uid) = combat_target.0
                && let Some(target_pos) = npc_pos.get(&target_uid).copied()
            {
                let dist = pos.distance(target_pos);
                let range = sim
                    .0
                    .npcs
                    .get(&npc_dot.uid)
                    .map(|n| weapon_range(items, &n.loadout))
                    .unwrap_or(0.0);
                if range > 0.0 && dist <= range {
                    // In range — stop walking so combat can fire.
                    if move_target.0.is_some() {
                        move_target.0 = None;
                    }
                } else {
                    // Out of range — close in.
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

        // === Not engaging: walk to formation slot. ===
        let Some(leader_p) = squad_leader_pos.get(&member.squad).copied() else {
            continue;
        };
        let facing = Vec2::new(squad.facing[0], squad.facing[1]).normalize_or_zero();
        if facing.length_squared() < 0.001 {
            continue;
        }
        let centroid = match activity {
            Activity::Hold { .. } => leader_p,
            Activity::Move { target } => *target,
            Activity::Engage { .. } => leader_p,
        };
        let offsets = squad.formation.slot_offsets(squad.members.len());
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

/// Set a member's [`MovementTarget`] only if it differs meaningfully
/// from the current value (avoids dirtying change-detection on every
/// tick when nothing actually moved).
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

// ====================================================================
// Squad lifecycle: leader promotion + empty-squad despawn.
// ====================================================================

/// Promote a new leader if the current one died, and despawn empty squads.
fn cleanup_dead_squads(
    sim: Option<ResMut<SimWorld>>,
    mut activities: ResMut<SquadActivities>,
) {
    let Some(mut sim) = sim else { return };
    let mut to_remove: Vec<Uid<Squad>> = Vec::new();

    let world = &mut sim.0;
    let npcs = &world.npcs;
    let squads = &mut world.squads;

    for (uid, squad) in squads.iter_mut() {
        squad.members.retain(|m| npcs.contains_key(m));
        if squad.members.is_empty() {
            to_remove.push(*uid);
            continue;
        }
        if !npcs.contains_key(&squad.leader) {
            let new_leader = squad
                .members
                .iter()
                .filter_map(|m| npcs.get(m).map(|n| (*m, n.rank())))
                .max_by_key(|(_, r)| *r)
                .map(|(m, _)| m);
            if let Some(new) = new_leader {
                squad.leader = new;
            } else {
                to_remove.push(*uid);
            }
        }
    }

    for uid in to_remove {
        squads.remove(&uid);
        activities.0.remove(&uid);
    }
}
