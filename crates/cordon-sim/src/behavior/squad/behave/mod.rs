//! Behavior-tree driven squad AI.
//!
//! Each squad has exactly one [`BehaveTree`] child, selected from its
//! current [`Goal`]. The tree writes [`MovementIntent`] — the squad's
//! "where do I want to be this frame" — and `drive_squad_formation`
//! turns that into per-member movement. [`EngagementTarget`] is owned
//! by the engagement scanner and read independently; engagement is
//! never suppressed by trees.
//!
//! Tree lifecycle:
//!
//! - Squad spawns → `Insert<Goal>` observer attaches the matching tree.
//! - Player/quest changes `Goal` via `insert` on the squad →
//!   observer despawns the old tree child and attaches a fresh one.
//! - Squad despawns → recursive despawn sweeps the tree child.
//!
//! Authoring rules for new trees:
//!
//! - Top-level is usually `Behave::Forever → Sequence` for looping
//!   goals. `GoTo` is a one-shot `Sequence` that terminates when the
//!   squad arrives.
//! - Action leaves (spawned components) clear `MovementIntent` on
//!   successful completion so formation treats the leader pos as the
//!   centroid afterwards. This keeps handoff between tree steps
//!   visually calm.
//! - Conditional checks use `Behave::trigger(T)` — the observer
//!   reports success/failure synchronously.

mod actions;
mod trees;

pub use actions::ActionsPlugin;
use bevy::ecs::lifecycle::Insert;
use bevy::prelude::*;
use bevy_behave::prelude::*;
use cordon_core::entity::squad::Goal;

use super::identity::SquadMarker;

/// Marker on the BehaveTree child entity so we can despawn it when
/// the squad's goal changes without touching sibling children.
#[derive(Component)]
pub(super) struct SquadTree;

/// Observer: whenever a `Goal` is inserted on a squad entity (either
/// at spawn or through a command-driven re-insert), despawn any
/// existing [`SquadTree`] child and attach a fresh one for the new
/// goal.
pub(super) fn attach_goal_tree(
    trigger: On<Insert, Goal>,
    squads_q: Query<&Goal, With<SquadMarker>>,
    children_q: Query<&Children>,
    trees_q: Query<(), With<SquadTree>>,
    mut commands: Commands,
) {
    let squad = trigger.event().entity;
    let Ok(goal) = squads_q.get(squad) else {
        return;
    };

    // Despawn any previous tree for this squad. There should be at
    // most one, but we iterate defensively.
    if let Ok(children) = children_q.get(squad) {
        for child in children.iter() {
            if trees_q.get(child).is_ok() {
                commands.entity(child).despawn();
            }
        }
    }

    let tree = trees::tree_for_goal(goal);
    commands.spawn((
        Name::new("SquadTree"),
        SquadTree,
        BehaveTree::new(tree),
        ChildOf(squad),
    ));
}
