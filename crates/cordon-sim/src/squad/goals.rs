//! Goal-driven activity transitions.
//!
//! When a squad's `Hold` timer expires, [`drive_squad_goals`] picks
//! the next [`SquadActivity`] from its long-term [`Goal`]. Patrol /
//! Scavenge cycle through their waypoint list; Idle and Protect just
//! re-arm a Hold.

use bevy::prelude::*;
use cordon_core::entity::squad::Goal;

use crate::components::{SquadActivity, SquadWaypoints};

pub(super) fn drive_squad_goals(
    time: Res<Time>,
    mut squads_q: Query<(&Goal, &mut SquadActivity, &mut SquadWaypoints)>,
) {
    let dt = time.delta_secs();
    for (goal, mut activity, mut waypoints) in &mut squads_q {
        if let SquadActivity::Hold { duration_secs } = &mut *activity {
            *duration_secs -= dt;
            if *duration_secs <= 0.0 {
                *activity = next_activity_for_goal(goal, &mut waypoints);
            }
        }
    }
}

fn next_activity_for_goal(goal: &Goal, waypoints: &mut SquadWaypoints) -> SquadActivity {
    match goal {
        Goal::Idle => SquadActivity::Hold { duration_secs: 4.0 },
        Goal::Patrol { .. } | Goal::Scavenge { .. } => {
            if waypoints.points.is_empty() {
                SquadActivity::Hold { duration_secs: 4.0 }
            } else {
                let idx = (waypoints.next as usize) % waypoints.points.len();
                let target = waypoints.points[idx];
                waypoints.next = ((idx + 1) % waypoints.points.len()) as u8;
                SquadActivity::Move { target }
            }
        }
        Goal::Protect { .. } => SquadActivity::Hold { duration_secs: 0.5 },
        _ => SquadActivity::Hold { duration_secs: 4.0 },
    }
}
