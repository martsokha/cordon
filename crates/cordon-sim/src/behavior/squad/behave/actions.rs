//! Behavior-tree action leaves for squad AI.
//!
//! Each leaf is a [`Component`] spawned by `Behave::spawn_named(...)`
//! when the tree enters that node. A dedicated system queries
//! `(&LeafComponent, &BehaveCtx)`, reads the target squad via
//! `ctx.target_entity()`, writes whatever ECS state the leaf
//! represents, and triggers `ctx.success()` / `ctx.failure()` when
//! done.
//!
//! Every movement leaf writes [`MovementIntent`] on entry and clears
//! it on successful completion, so the handoff between steps is
//! visually continuous.

use bevy::prelude::*;
use bevy_behave::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::Uid;

use crate::behavior::death::components::Dead;
use crate::entity::npc::NpcMarker;
use crate::resources::SquadIdIndex;
use super::super::formation::SquadWaypoints;
use super::super::identity::{SquadLeader, SquadMarker};
use super::super::intent::MovementIntent;
use super::super::constants::{ARRIVED_DIST, PROTECT_FOLLOW_DIST};

const FIND_SEARCH_RADIUS: f32 = 60.0;

/// Walk the squad's leader-anchored centroid toward a fixed point.
/// Succeeds when the leader is within [`ARRIVED_DIST`] of `target`.
/// Fails if the leader entity is missing / dead.
#[derive(Component, Clone, Debug)]
pub(super) struct BtMoveTo {
    pub target: Vec2,
}

/// Walk the cycling [`SquadWaypoints`] list. Each entry advances
/// through one waypoint, writes [`MovementIntent`], succeeds on
/// arrival, and bumps `next`. Succeeds immediately if the list is
/// empty (caller can wrap in a wait so patrol squads with no
/// authored waypoints don't spin).
#[derive(Component, Clone, Debug)]
pub(super) struct BtWalkWaypoint;

/// Follow another squad's leader. Writes [`MovementIntent`] toward
/// that leader's position each tick while the gap is larger than
/// [`PROTECT_FOLLOW_DIST`]; succeeds when the gap closes. Fails if
/// the other squad's leader can't be resolved (squad despawned /
/// leader dead / uid no longer indexed).
#[derive(Component, Clone, Debug)]
pub(super) struct BtProtectFollow {
    pub other: Uid<Squad>,
}

/// Hold in place for a fixed duration. Writes
/// `MovementIntent(None)` on entry so formation treats the leader
/// as the centroid, then succeeds after the timer expires.
///
/// Replaces the old `SquadActivity::Hold { duration_secs }` for the
/// Idle / post-patrol pause use case. bevy_behave's built-in
/// `Behave::Wait` can't clear `MovementIntent`, hence this custom
/// leaf.
#[derive(Component, Clone, Debug)]
pub(super) struct BtIdleHold {
    pub duration_secs: f32,
    pub elapsed_secs: f32,
}

impl BtIdleHold {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            duration_secs,
            elapsed_secs: 0.0,
        }
    }
}

/// Walk toward an NPC target by its stable [`Uid<Npc>`]. Succeeds
/// when the squad leader is within [`ARRIVED_DIST`] of the target's
/// current position (rescanned every tick so moving targets are
/// pursued). Fails if the target is missing / dead / despawned.
#[derive(Component, Clone, Debug)]
pub(super) struct BtFindNpc {
    pub target: Uid<Npc>,
    /// Last-seen position, written when the target is observed.
    /// Used as a fallback search point in the enclosing sequence if
    /// the target is no longer resolvable.
    pub last_seen: Option<Vec2>,
}

/// Plugin that registers every action tick system.
pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_move_to,
                tick_walk_waypoint,
                tick_protect_follow,
                tick_idle_hold,
                tick_find_npc,
            ),
        );
    }
}

fn tick_move_to(
    tasks: Query<(&BtMoveTo, &BehaveCtx)>,
    leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    for (move_to, ctx) in &tasks {
        let squad = ctx.target_entity();
        let Ok(leader) = leaders_q.get(squad) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(leader_t) = transforms_q.get(leader.0) else {
            commands.trigger(ctx.failure());
            continue;
        };
        if let Ok(mut intent) = intents_q.get_mut(squad)
            && intent.0 != Some(move_to.target)
        {
            intent.0 = Some(move_to.target);
        }
        let leader_pos = leader_t.translation.truncate();
        if leader_pos.distance(move_to.target) < ARRIVED_DIST {
            if let Ok(mut intent) = intents_q.get_mut(squad) {
                intent.0 = None;
            }
            commands.trigger(ctx.success());
        }
    }
}

fn tick_walk_waypoint(
    tasks: Query<(&BtWalkWaypoint, &BehaveCtx)>,
    leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut squads_q: Query<(&mut MovementIntent, &mut SquadWaypoints)>,
    mut commands: Commands,
) {
    for (_, ctx) in &tasks {
        let squad = ctx.target_entity();
        let Ok((mut intent, mut waypoints)) = squads_q.get_mut(squad) else {
            commands.trigger(ctx.failure());
            continue;
        };
        if waypoints.points.is_empty() {
            // Nothing to do — succeed immediately so the enclosing
            // sequence can run its post-waypoint Wait and the tree
            // continues to tick. Without this, a Patrol squad with
            // empty waypoints would stall forever.
            if intent.0.is_some() {
                intent.0 = None;
            }
            commands.trigger(ctx.success());
            continue;
        }
        let idx = (waypoints.next as usize) % waypoints.points.len();
        let target = waypoints.points[idx];
        if intent.0 != Some(target) {
            intent.0 = Some(target);
        }
        let Ok(leader) = leaders_q.get(squad) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(leader_t) = transforms_q.get(leader.0) else {
            // Leader dead or missing — the tree leaf can't make
            // progress this tick. Fail; the enclosing Forever will
            // restart the sequence next frame, by which time
            // lifecycle should have promoted a new leader.
            commands.trigger(ctx.failure());
            continue;
        };
        let leader_pos = leader_t.translation.truncate();
        if leader_pos.distance(target) < ARRIVED_DIST {
            waypoints.next = ((idx + 1) % waypoints.points.len()) as u8;
            intent.0 = None;
            commands.trigger(ctx.success());
        }
    }
}

fn tick_protect_follow(
    tasks: Query<(&BtProtectFollow, &BehaveCtx)>,
    squad_index: Res<SquadIdIndex>,
    leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    for (follow, ctx) in &tasks {
        let squad = ctx.target_entity();
        let Some(other_entity) = squad_index.0.get(&follow.other).copied() else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(other_leader) = leaders_q.get(other_entity) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(other_t) = transforms_q.get(other_leader.0) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(own_leader) = leaders_q.get(squad) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(own_t) = transforms_q.get(own_leader.0) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let target = other_t.translation.truncate();
        let own_pos = own_t.translation.truncate();
        if own_pos.distance(target) <= PROTECT_FOLLOW_DIST {
            if let Ok(mut intent) = intents_q.get_mut(squad) {
                intent.0 = None;
            }
            commands.trigger(ctx.success());
            continue;
        }
        if let Ok(mut intent) = intents_q.get_mut(squad)
            && intent.0 != Some(target)
        {
            intent.0 = Some(target);
        }
    }
}

fn tick_idle_hold(
    time: Res<Time>,
    mut tasks: Query<(&mut BtIdleHold, &BehaveCtx)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (mut hold, ctx) in &mut tasks {
        if hold.elapsed_secs == 0.0
            && let Ok(mut intent) = intents_q.get_mut(ctx.target_entity())
            && intent.0.is_some()
        {
            intent.0 = None;
        }
        hold.elapsed_secs += dt;
        if hold.elapsed_secs >= hold.duration_secs {
            commands.trigger(ctx.success());
        }
    }
}

fn tick_find_npc(
    tasks: Query<(&BtFindNpc, &BehaveCtx)>,
    leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    npc_q: Query<(&Uid<Npc>, &Transform), (With<NpcMarker>, Without<Dead>)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    for (find, ctx) in &tasks {
        let squad = ctx.target_entity();
        let Ok(leader) = leaders_q.get(squad) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Ok(leader_t) = transforms_q.get(leader.0) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let leader_pos = leader_t.translation.truncate();

        // Try to resolve the current position of the target npc by
        // its stable Uid. O(N) over alive npcs — fine at our
        // population sizes; if it ever bites we can index by Uid
        // separately.
        let target_pos = npc_q
            .iter()
            .find(|(uid, _)| **uid == find.target)
            .map(|(_, t)| t.translation.truncate());

        let walk_to = match (target_pos, find.last_seen) {
            (Some(p), _) => {
                // Target is alive and visible somewhere — walk to them.
                // last_seen should be updated, but BehaveCtx queries are
                // immutable so that's deferred to a separate system;
                // for now the fresh position is enough.
                Some(p)
            }
            (None, Some(last)) => {
                // Target unresolvable this tick — sweep the last-seen
                // area, succeeding once the leader is within search
                // radius to trigger re-acquisition next frame.
                if leader_pos.distance(last) < FIND_SEARCH_RADIUS {
                    // Arrived at last-seen, target still missing —
                    // fail so the enclosing fallback can take
                    // another branch (e.g. patrol more waypoints).
                    if let Ok(mut intent) = intents_q.get_mut(squad) {
                        intent.0 = None;
                    }
                    commands.trigger(ctx.failure());
                    continue;
                }
                Some(last)
            }
            (None, None) => {
                // No target, no last-seen — authoring error. Fail so
                // the enclosing fallback moves on.
                if let Ok(mut intent) = intents_q.get_mut(squad) {
                    intent.0 = None;
                }
                commands.trigger(ctx.failure());
                continue;
            }
        };

        if let Some(target) = walk_to
            && let Ok(mut intent) = intents_q.get_mut(squad)
            && intent.0 != Some(target)
        {
            intent.0 = Some(target);
        }

        if let Some(target) = walk_to
            && leader_pos.distance(target) < ARRIVED_DIST
        {
            if let Ok(mut intent) = intents_q.get_mut(squad) {
                intent.0 = None;
            }
            commands.trigger(ctx.success());
        }
    }
}
