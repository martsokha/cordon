//! Behavior-tree integration pilot (Idle goal only).
//!
//! Wires `bevy_behave` into the squad pipeline without touching the
//! existing `SquadActivity` FSM yet. When a squad spawns with
//! [`Goal::Idle`](cordon_core::entity::squad::Goal::Idle) we attach a
//! child entity carrying a minimal [`BehaveTree`] that just ticks a
//! forever-wait. The goal here is to prove the integration end-to-end
//! — plugin registration, tree spawn lifecycle, recursive cleanup —
//! before porting any goal that actually drives movement or combat.
//!
//! Non-Idle squads are left entirely alone by this module.

use bevy::prelude::*;
use bevy_behave::prelude::*;
use cordon_core::entity::squad::Goal;

use crate::components::SquadMarker;

/// Attach a behavior tree to newly-spawned squads whose initial goal
/// is [`Goal::Idle`]. Runs as an add-observer so the tree lands on the
/// entity in the same command flush as the `SquadBundle`.
///
/// Deliberately scoped to `Goal::Idle`: other goals still flow through
/// the hand-rolled [`SquadActivity`](crate::components::SquadActivity)
/// FSM. Expanding this to more goals is the next step once the pilot
/// is verified in-game.
pub(super) fn attach_idle_tree(
    trigger: On<Add, SquadMarker>,
    goals_q: Query<&Goal>,
    mut commands: Commands,
) {
    let squad = trigger.event().entity;
    let Ok(goal) = goals_q.get(squad) else {
        return;
    };
    if !matches!(goal, Goal::Idle) {
        return;
    }
    let tree = behave! {
        Behave::Forever => {
            Behave::Wait(4.0)
        }
    };
    commands.spawn((
        Name::new("SquadIdleTree"),
        BehaveTree::new(tree),
        ChildOf(squad),
    ));
}
