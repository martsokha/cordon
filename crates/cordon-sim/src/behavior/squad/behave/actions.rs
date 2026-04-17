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
//! visually continuous. Two small helpers — [`leader_pos`] and
//! [`clear_intent`] — dedupe the repeated ECS lookups each leaf
//! would otherwise spell out by hand.

use bevy::prelude::*;
use bevy_behave::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::Uid;

use super::super::constants::{ARRIVED_DIST, PROTECT_FOLLOW_DIST};
use super::super::formation::SquadWaypoints;
use super::super::identity::{SquadLeader, SquadMarker};
use super::super::intent::MovementIntent;
use crate::behavior::death::components::Dead;
use crate::entity::npc::NpcMarker;
use crate::resources::SquadIdIndex;

/// How close the squad leader must get to a last-seen position
/// before [`BtFindNpc`] fails over to the enclosing fallback.
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
    /// Last-seen position, written each tick we can resolve the
    /// target. Used as a fallback search point if the target later
    /// disappears.
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

/// Resolve the current position of a squad's leader by chasing
/// the [`SquadLeader`] pointer to its NPC transform. Returns
/// `None` if the squad isn't queryable, the leader entity is
/// missing, or the leader is tagged `Dead` (the transforms query
/// filters out corpses).
///
/// This pattern appears verbatim in every action leaf; centralising
/// it keeps the tick logic focused on decisions, not lookups.
fn leader_pos(
    squad: Entity,
    leaders_q: &Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: &Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
) -> Option<Vec2> {
    let leader = leaders_q.get(squad).ok()?;
    let transform = transforms_q.get(leader.0).ok()?;
    Some(transform.translation.truncate())
}

/// Clear the squad's [`MovementIntent`] so formation stops
/// pulling members toward the last target. Silent no-op if the
/// squad has no intent component (shouldn't happen, but defensive).
fn clear_intent(intents_q: &mut Query<&mut MovementIntent>, squad: Entity) {
    if let Ok(mut intent) = intents_q.get_mut(squad)
        && intent.0.is_some()
    {
        intent.0 = None;
    }
}

/// Set the squad's [`MovementIntent`] to `target` if it differs
/// from the current value. Change-only write avoids dirtying
/// Bevy's change detection for every re-tick of the same target.
fn set_intent(intents_q: &mut Query<&mut MovementIntent>, squad: Entity, target: Vec2) {
    if let Ok(mut intent) = intents_q.get_mut(squad)
        && intent.0 != Some(target)
    {
        intent.0 = Some(target);
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
        let Some(leader_pos) = leader_pos(squad, &leaders_q, &transforms_q) else {
            commands.trigger(ctx.failure());
            continue;
        };
        set_intent(&mut intents_q, squad, move_to.target);
        if leader_pos.distance(move_to.target) < ARRIVED_DIST {
            clear_intent(&mut intents_q, squad);
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
        // Can't use `leader_pos` here because we're already
        // holding `squads_q` mutably on `squad`; resolving the leader
        // through a separate read-only query doesn't conflict.
        let Some(leader_pos) = leader_pos(squad, &leaders_q, &transforms_q) else {
            // Leader dead or missing — the tree leaf can't make
            // progress this tick. Fail; the enclosing Forever will
            // restart the sequence next frame, by which time
            // lifecycle should have promoted a new leader.
            commands.trigger(ctx.failure());
            continue;
        };
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
        let Some(target) = leader_pos(other_entity, &leaders_q, &transforms_q) else {
            commands.trigger(ctx.failure());
            continue;
        };
        let Some(own_pos) = leader_pos(squad, &leaders_q, &transforms_q) else {
            commands.trigger(ctx.failure());
            continue;
        };
        if own_pos.distance(target) <= PROTECT_FOLLOW_DIST {
            clear_intent(&mut intents_q, squad);
            commands.trigger(ctx.success());
            continue;
        }
        set_intent(&mut intents_q, squad, target);
    }
}

fn tick_idle_hold(
    time: Res<Time<crate::resources::Sim>>,
    mut tasks: Query<(&mut BtIdleHold, &BehaveCtx)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (mut hold, ctx) in &mut tasks {
        if hold.elapsed_secs == 0.0 {
            clear_intent(&mut intents_q, ctx.target_entity());
        }
        hold.elapsed_secs += dt;
        if hold.elapsed_secs >= hold.duration_secs {
            commands.trigger(ctx.success());
        }
    }
}

fn tick_find_npc(
    mut tasks: Query<(&mut BtFindNpc, &BehaveCtx)>,
    leaders_q: Query<&SquadLeader, With<SquadMarker>>,
    transforms_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    npc_q: Query<(&Uid<Npc>, &Transform), (With<NpcMarker>, Without<Dead>)>,
    mut intents_q: Query<&mut MovementIntent>,
    mut commands: Commands,
) {
    for (mut find, ctx) in &mut tasks {
        let squad = ctx.target_entity();
        let Some(leader_pos) = leader_pos(squad, &leaders_q, &transforms_q) else {
            commands.trigger(ctx.failure());
            continue;
        };

        // Try to resolve the current position of the target npc by
        // its stable Uid. O(N) over alive npcs — fine at our
        // population sizes; if it ever bites we can index by Uid
        // separately.
        let target_pos = npc_q
            .iter()
            .find(|(uid, _)| **uid == find.target)
            .map(|(_, t)| t.translation.truncate());

        // Record sightings so a target that later disappears still
        // has a fallback point to sweep toward.
        if let Some(p) = target_pos {
            find.last_seen = Some(p);
        }

        let walk_to = match (target_pos, find.last_seen) {
            // Target is alive and visible — walk to the fresh
            // position; last_seen was just updated above.
            (Some(p), _) => Some(p),
            // Target unresolvable this tick — sweep the last-seen
            // area, failing once we arrive there still empty-handed
            // so the enclosing fallback can take another branch.
            (None, Some(last)) => {
                if leader_pos.distance(last) < FIND_SEARCH_RADIUS {
                    clear_intent(&mut intents_q, squad);
                    commands.trigger(ctx.failure());
                    continue;
                }
                Some(last)
            }
            // No target, never seen — authoring error.
            (None, None) => {
                clear_intent(&mut intents_q, squad);
                commands.trigger(ctx.failure());
                continue;
            }
        };

        if let Some(target) = walk_to {
            set_intent(&mut intents_q, squad, target);
            if leader_pos.distance(target) < ARRIVED_DIST {
                clear_intent(&mut intents_q, squad);
                commands.trigger(ctx.success());
            }
        }
    }
}
