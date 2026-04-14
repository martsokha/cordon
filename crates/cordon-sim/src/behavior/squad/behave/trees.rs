//! Goal → tree factory.
//!
//! Each [`Goal`] variant maps to a tree shape that drives the
//! squad's [`MovementIntent`]. Trees are constructed fresh on every
//! goal insertion (see `attach_goal_tree`), so this module is pure
//! — no state carried between calls.

use bevy::prelude::*;
use bevy_behave::prelude::*;
use cordon_core::entity::squad::Goal;

use super::actions::{BtFindNpc, BtIdleHold, BtMoveTo, BtProtectFollow, BtWalkWaypoint};
use super::super::constants::PATROL_HOLD_SECS;

const IDLE_HOLD_SECS: f32 = 4.0;

/// Build the behavior tree for a given goal. Called from
/// `attach_goal_tree` on every `Insert<Goal>` trigger.
pub(super) fn tree_for_goal(goal: &Goal) -> Tree<Behave> {
    match goal {
        Goal::Idle => idle_tree(),
        Goal::Patrol { .. } | Goal::Scavenge { .. } => patrol_tree(),
        Goal::Protect { other } => protect_tree(*other),
        Goal::GoTo { target, .. } => goto_tree(Vec2::new(target[0], target[1])),
        Goal::Find { target } => find_tree(*target),
    }
}

fn idle_tree() -> Tree<Behave> {
    behave! {
        Behave::Forever => {
            Behave::spawn_named("IdleHold", BtIdleHold::new(IDLE_HOLD_SECS))
        }
    }
}

fn patrol_tree() -> Tree<Behave> {
    // Walk next waypoint, pause at it, loop. A dedicated hold leaf
    // (not Behave::Wait) so MovementIntent clears and the squad
    // visibly stops at the waypoint instead of drifting on.
    behave! {
        Behave::Forever => {
            Behave::Sequence => {
                Behave::spawn_named("WalkWaypoint", BtWalkWaypoint),
                Behave::spawn_named("PatrolHold", BtIdleHold::new(PATROL_HOLD_SECS))
            }
        }
    }
}

fn protect_tree(other: cordon_core::primitive::Uid<cordon_core::entity::squad::Squad>) -> Tree<Behave> {
    // Close the gap, pause briefly, re-evaluate. The follow leaf
    // succeeds as soon as we're within PROTECT_FOLLOW_DIST, so the
    // loop rebuilds intent every half-second once in range — cheap,
    // and lets us react quickly if the protected squad moves.
    behave! {
        Behave::Forever => {
            Behave::Sequence => {
                Behave::spawn_named("ProtectFollow", BtProtectFollow { other }),
                Behave::spawn_named("ProtectSettle", BtIdleHold::new(0.5))
            }
        }
    }
}

fn goto_tree(target: Vec2) -> Tree<Behave> {
    // One-shot: walk to the target, then hold. No Forever — once we
    // arrive, the tree terminates and the squad idles until something
    // else (a player command, an arrival handler) changes Goal.
    behave! {
        Behave::Sequence => {
            Behave::spawn_named("GoToWalk", BtMoveTo { target }),
            Behave::spawn_named("GoToSettle", BtIdleHold::new(IDLE_HOLD_SECS))
        }
    }
}

fn find_tree(target: cordon_core::primitive::Uid<cordon_core::entity::npc::Npc>) -> Tree<Behave> {
    // Chase the target while resolvable; when the target disappears
    // the leaf fails on arrival at last-seen, so the enclosing
    // Forever restarts the search next frame.
    behave! {
        Behave::Forever => {
            Behave::spawn_named("FindNpc", BtFindNpc { target, last_seen: None })
        }
    }
}
