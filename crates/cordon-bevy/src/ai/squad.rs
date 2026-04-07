//! Squad ECS layer: per-NPC SquadMember component, formation
//! positioning, goal-driven activity, vision sharing, focus fire, and
//! squad lifecycle (leader promotion + cleanup).
//!
//! The squad data itself lives in [`cordon_sim::state::world::World::squads`].
//! Per-NPC entities carry a [`SquadMember`] back-pointer so queries can
//! find an NPC's squad in O(1) without scanning the hashmap.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::{Goal, Squad};
use cordon_core::primitive::Uid;
use cordon_data::gamedata::GameDataResource;
use moonshine_behavior::prelude::*;

use super::behavior::Action;
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

/// What the squad is currently doing. The activity is short-term:
/// hold a position, walk somewhere in formation, or focus fire on a
/// hostile squad. Goals are long-term reasons (`Squad::goal`).
#[derive(Component, Debug, Clone)]
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
/// How long a squad holds at a patrol waypoint before moving on.
const PATROL_HOLD_SECS: f32 = 6.0;
/// Distance below which a squad member considers their formation slot
/// reached. Generous so the formation system doesn't oscillate between
/// Walk and Idle on tiny position deltas.
const ARRIVED_DIST: f32 = 12.0;

/// Plugin registering the squad systems.
pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadActivities>();
        app.add_systems(
            Update,
            (
                update_squad_engagement,
                drive_squad_formation,
                drive_squad_goals,
                cleanup_dead_squads,
            )
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Per-squad activity stored as a Bevy resource. We don't put squads
/// on entities for v1; instead this map keys by squad uid.
#[derive(Resource, Default)]
pub struct SquadActivities(pub HashMap<Uid<Squad>, Activity>);

/// One snapshot of an alive NPC's spatial state, used by the
/// engagement scanner.
struct NpcSnap {
    uid: Uid<Npc>,
    squad: Uid<Squad>,
    pos: Vec2,
    vision: f32,
    weapon_range: f32,
}

/// 2D spatial hash grid mapping cell coordinates to NPC snapshot
/// indices. Used by `update_squad_engagement` to skip far-away NPCs
/// without iterating the whole world.
type SpatialGrid = HashMap<(i32, i32), Vec<usize>>;

/// Build a grid that bins each snapshot index into its cell.
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

/// Push every snapshot index from cells overlapping the disc
/// `(center, radius)` into `out`.
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


/// Each tick, scan for hostile-squad sightings via shared squad vision.
/// When a squad spots a hostile, set its activity to `Engage { hostiles }`
/// and assign each member their own nearest enemy from that squad to
/// shoot at. When no hostile is in sight, drop back to Hold/Move.
#[allow(clippy::type_complexity)]
fn update_squad_engagement(
    sim: Option<ResMut<SimWorld>>,
    game_data: Res<GameDataResource>,
    mut activities: ResMut<SquadActivities>,
    anomalies: Query<(&Transform, &AnomalyZone)>,
    members_q: Query<
        (&NpcDot, &SquadMember, &Vision, &Transform),
        Without<Dead>,
    >,
    mut behaviors_q: Query<(&NpcDot, BehaviorMut<Action>), Without<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let factions = &game_data.0.factions;
    let items = &game_data.0.items;

    let anomaly_disks: Vec<(Vec2, f32)> = anomalies
        .iter()
        .map(|(t, a)| (t.translation.truncate(), a.radius))
        .collect();

    // Build a snapshot of every alive NPC: position, vision, weapon range,
    // and squad uid.
    let mut snapshot: Vec<NpcSnap> = Vec::with_capacity(members_q.iter().count());
    for (dot, member, vision, transform) in &members_q {
        let Some(npc) = sim.0.npcs.get(&dot.uid) else {
            continue;
        };
        let range = weapon_range(items, &npc.loadout);
        snapshot.push(NpcSnap {
            uid: dot.uid,
            squad: member.squad,
            pos: transform.translation.truncate(),
            vision: vision.radius,
            weapon_range: range,
        });
    }

    // === Spatial grid ===
    // Bucket every NPC into a 2D grid of `CELL_SIZE` map units. Each
    // squad's vision scan only checks NPCs in nearby cells, instead of
    // every NPC in the world.
    const CELL_SIZE: f32 = 200.0;
    let grid = build_spatial_grid(&snapshot, CELL_SIZE);

    // For each squad, decide which hostile squad it's engaging (if any).
    // A squad's "shared vision" is satisfied if any of its members can
    // see (in vision + LOS) any NPC from a hostile squad.
    let mut squad_hostile: HashMap<Uid<Squad>, Uid<Squad>> = HashMap::new();
    for (squad_uid, squad) in &sim.0.squads {
        // Members of this squad in the snapshot.
        let members: Vec<&NpcSnap> =
            snapshot.iter().filter(|n| n.squad == *squad_uid).collect();
        if members.is_empty() {
            continue;
        }

        // Maximum vision among our members; defines the search radius.
        let max_vision = members.iter().map(|m| m.vision).fold(0.0_f32, f32::max);

        // Collect candidate NPC indices from grid cells overlapping
        // any member's vision disc.
        let mut candidates: Vec<usize> = Vec::new();
        for m in &members {
            collect_nearby_cells(
                m.pos,
                m.vision,
                &grid,
                CELL_SIZE,
                &mut candidates,
            );
        }
        // Deduplicate (a single NPC may appear in multiple members'
        // searches). Sort + dedup is fastest for small Vecs.
        candidates.sort_unstable();
        candidates.dedup();

        let mut chosen: Option<(Uid<Squad>, f32)> = None;
        for cand_idx in candidates {
            let cand = &snapshot[cand_idx];
            if cand.squad == *squad_uid {
                continue;
            }
            // Hostility is per-faction; look up the candidate squad's
            // faction via sim.
            let Some(cand_squad) = sim.0.squads.get(&cand.squad) else {
                continue;
            };
            if !is_hostile(&squad.faction, &cand_squad.faction, factions) {
                continue;
            }
            // Any of our members can see them? Squared-distance check
            // first, then LOS only if the cheap check passes.
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
            let dist = dist_sq.sqrt();

            // Pick the closest visible hostile *squad* (not NPC).
            if chosen.is_none_or(|(_, d)| dist < d) {
                chosen = Some((cand.squad, dist));
            }
        }
        let _ = max_vision;

        if let Some((hostile_squad_uid, _)) = chosen {
            squad_hostile.insert(*squad_uid, hostile_squad_uid);
        }
    }

    // Snapshot every squad's leader uid so we can read it during the
    // mutable iter_mut below without re-borrowing sim.
    let leader_by_squad: HashMap<Uid<Squad>, Uid<Npc>> = sim
        .0
        .squads
        .iter()
        .map(|(uid, sq)| (*uid, sq.leader))
        .collect();

    // Update activities + facing: squads with a hostile go into Engage
    // and rotate to face the hostile leader; squads without one lose
    // their Engage activity.
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
                // Point this squad's facing toward the leader of the
                // hostile squad we're engaging.
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
                // Drop out of Engage if we're no longer seeing anyone hostile.
                if matches!(activity, Activity::Engage { .. }) {
                    *activity = Activity::Hold { duration_secs: 0.5 };
                }
            }
        }
    }

    // For each member of an engaging squad, pick their own nearest
    // enemy from the hostile squad they can reach (or get close to).
    let snapshot_by_squad: HashMap<Uid<Squad>, Vec<&NpcSnap>> = {
        let mut m: HashMap<Uid<Squad>, Vec<&NpcSnap>> = HashMap::new();
        for n in &snapshot {
            m.entry(n.squad).or_default().push(n);
        }
        m
    };

    for (npc_dot, mut behavior) in &mut behaviors_q {
        let Some(snap) = snapshot.iter().find(|n| n.uid == npc_dot.uid) else {
            continue;
        };
        let Some(activity) = activities.0.get(&snap.squad) else {
            continue;
        };
        let Activity::Engage { hostiles } = activity else {
            // Not engaging — let formation system push our walk action.
            // If we *were* engaged, drop the Action::Engage so we can
            // resume normal movement.
            if matches!(behavior.current(), Action::Engage { .. }) {
                let _ = behavior.try_start(Action::Idle { timer: 0.5 });
            }
            continue;
        };

        // Find this NPC's nearest reachable enemy in the hostile squad.
        let Some(hostile_members) = snapshot_by_squad.get(hostiles) else {
            continue;
        };
        let mut best: Option<(Uid<Npc>, Vec2, f32)> = None;
        for enemy in hostile_members {
            if line_blocked(snap.pos, enemy.pos, &anomaly_disks) {
                continue;
            }
            let dist = snap.pos.distance(enemy.pos);
            if best.is_none_or(|(_, _, d)| dist < d) {
                best = Some((enemy.uid, enemy.pos, dist));
            }
        }
        let Some((target_uid, target_pos, dist)) = best else {
            continue;
        };

        if snap.weapon_range > 0.0 && dist <= snap.weapon_range {
            // In weapon range: enter or update Engage with this target.
            let already = matches!(
                behavior.current(),
                Action::Engage { target, .. } if *target == target_uid
            );
            if !already {
                let _ = behavior.try_start(Action::Engage {
                    target: target_uid,
                    cooldown_secs: 0.0,
                    reload_secs: 0.0,
                });
            }
        } else {
            // Out of range: walk toward the chosen enemy. The formation
            // system will be overridden by this Walk push.
            let already = matches!(
                behavior.current(),
                Action::Walk { target, .. } if target.distance(target_pos) < 4.0
            );
            if !already {
                let _ = behavior.try_start(Action::Walk {
                    target: target_pos,
                    speed: 35.0,
                });
            }
        }
    }

}

/// Pick the next activity for each squad based on its goal.
///
/// For v1:
///   - Idle: keeps holding forever
///   - Patrol/Scavenge: cycle through the squad's waypoints inside the area
///   - Protect: head toward the protected squad's leader
///
/// Activity transitions caused by *arrival* (Move → Hold) live in
/// [`drive_squad_formation`], which has the leader positions in scope.
fn drive_squad_goals(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    mut activities: ResMut<SquadActivities>,
) {
    let Some(mut sim) = sim else { return };
    let dt = time.delta_secs();

    // Snapshot leaders' goals + waypoint state so we can mutate squads
    // and read other squads' positions in one pass.
    let leader_uids: HashMap<Uid<Squad>, Uid<Npc>> = sim
        .0
        .squads
        .iter()
        .map(|(uid, sq)| (*uid, sq.leader))
        .collect();

    for (squad_uid, squad) in sim.0.squads.iter_mut() {
        if squad.members.is_empty() {
            continue;
        }
        let activity = activities.0.entry(*squad_uid).or_insert(Activity::Hold {
            duration_secs: 1.0,
        });

        if let Activity::Hold { duration_secs } = activity {
            *duration_secs -= dt;
            if *duration_secs <= 0.0 {
                *activity = next_activity_for_squad(squad, &leader_uids);
            }
        }
    }

    // Note: Protect's "follow the protected squad's leader" needs the
    // leader's *current position*, which lives on the ECS Transform.
    // The formation system handles per-tick updates by reading positions
    // there and re-pointing the Move target when the protect-target
    // moves.
}

/// Compute the next activity for a squad based on its goal.
fn next_activity_for_squad(
    squad: &mut Squad,
    leader_uids: &HashMap<Uid<Squad>, Uid<Npc>>,
) -> Activity {
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
        Goal::Protect { other } => {
            // Without per-tick position, we can't pick a target here.
            // Fall through to a brief hold; the formation system bumps
            // the Move target to the protected squad's leader each tick.
            let _ = leader_uids.get(other);
            Activity::Hold {
                duration_secs: 0.5,
            }
        }
        _ => Activity::Hold {
            duration_secs: 4.0,
        },
    }
}

/// Each tick:
///  1. Snapshot leader positions per squad
///  2. For Protect goals, set the Move target to the protected squad's leader
///  3. Flip arrived Move activities to Hold
///  4. Update each squad's facing (toward Move target, or kept stable)
///  5. Push `Action::Walk` onto every formation member (skipped for
///     Engage activities — combat does its own walking)
///
/// Throttled to ~10Hz: formation positions only need to refresh as
/// fast as the eye can perceive, and the per-frame query iteration
/// over ~1000 NPCs was a hot path at large populations.
#[allow(clippy::type_complexity)]
fn drive_squad_formation(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut activities: ResMut<SquadActivities>,
    mut members_q: Query<(&NpcDot, &SquadMember, &Transform, BehaviorMut<Action>), Without<Dead>>,
    leaders_q: Query<(&NpcDot, &Transform), Without<Dead>>,
) {
    const FORMATION_INTERVAL_SECS: f32 = 0.1;
    *throttle += time.delta_secs();
    if *throttle < FORMATION_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let Some(mut sim) = sim else { return };

    // Build leader_uid → Vec2 lookup once.
    let leader_pos: HashMap<Uid<Npc>, Vec2> = leaders_q
        .iter()
        .map(|(dot, t)| (dot.uid, t.translation.truncate()))
        .collect();

    // Snapshot squad leader positions for cross-squad reads.
    let mut squad_leader_pos: HashMap<Uid<Squad>, Vec2> = HashMap::new();
    for (squad_uid, squad) in &sim.0.squads {
        if let Some(p) = leader_pos.get(&squad.leader).copied() {
            squad_leader_pos.insert(*squad_uid, p);
        }
    }

    // Pass A: update activities + facing per squad.
    for (squad_uid, squad) in sim.0.squads.iter_mut() {
        let Some(p) = squad_leader_pos.get(squad_uid).copied() else {
            continue;
        };

        // Goal::Protect overrides the activity each tick to track the
        // protected squad's leader.
        if let Goal::Protect { other } = &squad.goal
            && let Some(other_pos) = squad_leader_pos.get(other).copied()
        {
            // Stay slightly off the protected leader so the formation
            // doesn't overlap them.
            let offset_target = other_pos;
            activities
                .0
                .insert(*squad_uid, Activity::Move { target: offset_target });
        }

        // Flip arrived Move activities to Hold (only if we're not in
        // an Engage that overrides movement).
        if let Some(activity) = activities.0.get_mut(squad_uid)
            && let Activity::Move { target } = activity
            && p.distance(*target) < ARRIVED_DIST
        {
            *activity = Activity::Hold {
                duration_secs: PATROL_HOLD_SECS,
            };
        }

        // Update facing — even when holding, point toward the most
        // relevant direction we know about.
        let new_facing = match activities.0.get(squad_uid) {
            Some(Activity::Move { target }) => (*target - p).normalize_or_zero(),
            _ => Vec2::new(squad.facing[0], squad.facing[1]),
        };
        if new_facing.length_squared() > 0.001 {
            squad.facing = [new_facing.x, new_facing.y];
        } else if squad.facing == [0.0, 0.0] {
            // Brand new squad with no movement yet — point north.
            squad.facing = [0.0, 1.0];
        }
    }

    // Pass B: dispatch Walk targets to each member based on their slot.
    // Skipped for squads in Engage (combat owns those members' walks).
    for (_npc_dot, member, transform, mut behavior) in &mut members_q {
        let Some(squad) = sim.0.squads.get(&member.squad) else {
            continue;
        };
        let Some(activity) = activities.0.get(&member.squad) else {
            continue;
        };
        if matches!(activity, Activity::Engage { .. }) {
            continue;
        }
        let Some(leader_p) = squad_leader_pos.get(&member.squad).copied() else {
            continue;
        };
        let facing = Vec2::new(squad.facing[0], squad.facing[1]).normalize_or_zero();
        if facing.length_squared() < 0.001 {
            continue;
        }

        // Where should this squad's centroid be standing right now?
        let centroid = match activity {
            Activity::Hold { .. } => leader_p,
            Activity::Move { target } => *target,
            Activity::Engage { .. } => leader_p,
        };

        // Compute the slot offset for this member's slot in the
        // current formation.
        let offsets = squad.formation.slot_offsets(squad.members.len());
        let slot = (member.slot as usize).min(offsets.len().saturating_sub(1));
        let local = Vec2::new(offsets[slot][0], offsets[slot][1]);

        // Rotate the local offset by the squad's facing.
        let perp = Vec2::new(-facing.y, facing.x);
        let world_offset = perp * local.x + facing * local.y;
        let target = centroid + world_offset;

        // If we're far from the target, push a Walk; otherwise sit idle.
        let pos = transform.translation.truncate();
        let dist = pos.distance(target);
        if dist > ARRIVED_DIST {
            // Avoid spamming new Walks every tick if we're already
            // walking to roughly the same place.
            let should_push = match behavior.current() {
                Action::Walk { target: t, .. } => t.distance(target) > ARRIVED_DIST,
                Action::Engage { .. } | Action::Loot { .. } | Action::Flee { .. } => false,
                _ => true,
            };
            if should_push {
                let _ = behavior.try_start(Action::Walk {
                    target,
                    speed: SQUAD_WALK_SPEED,
                });
            }
        } else if matches!(behavior.current(), Action::Walk { .. }) {
            // Reached our spot — relax to idle so we don't keep walking.
            let _ = behavior.try_start(Action::Idle { timer: 0.5 });
        }
    }
}

/// Promote a new leader if the current one died, and despawn empty squads.
fn cleanup_dead_squads(
    sim: Option<ResMut<SimWorld>>,
    mut activities: ResMut<SquadActivities>,
) {
    let Some(mut sim) = sim else { return };
    let mut to_remove: Vec<Uid<Squad>> = Vec::new();

    // Split-borrow the world's two maps so we can iterate squads
    // mutably while reading npcs immutably.
    let world = &mut sim.0;
    let npcs = &world.npcs;
    let squads = &mut world.squads;

    for (uid, squad) in squads.iter_mut() {
        // Drop members that no longer exist (despawned by cleanup_corpses).
        squad.members.retain(|m| npcs.contains_key(m));
        if squad.members.is_empty() {
            to_remove.push(*uid);
            continue;
        }

        // Leader still alive?
        if !npcs.contains_key(&squad.leader) {
            // Promote highest-rank survivor.
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
