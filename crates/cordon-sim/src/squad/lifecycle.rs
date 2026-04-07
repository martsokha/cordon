//! Squad lifecycle: prune dead members from the squad's member list
//! and promote new leaders when the old one dies. Throttled to 1 Hz.
//!
//! Squad entities are never despawned. A squad with no alive members
//! sits inert until the runtime decides otherwise — corpses of its
//! members persist independently and stay lootable through the
//! separate corpse cleanup path.

use bevy::prelude::*;

use crate::behavior::Dead;
use crate::components::{NpcMarker, SquadLeader, SquadMembers, Xp};
use crate::tuning::CLEANUP_INTERVAL_SECS;

pub(super) fn cleanup_dead_squads(
    time: Res<Time>,
    mut throttle: Local<f32>,
    alive_q: Query<&Xp, (With<NpcMarker>, Without<Dead>)>,
    mut squads_q: Query<(&mut SquadMembers, &mut SquadLeader)>,
) {
    *throttle += time.delta_secs();
    if *throttle < CLEANUP_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    for (mut members, mut leader) in &mut squads_q {
        members.0.retain(|m| alive_q.get(*m).is_ok());
        if members.0.is_empty() {
            continue;
        }
        if alive_q.get(leader.0).is_err() {
            // Promote the highest-rank surviving member.
            if let Some(new) = members
                .0
                .iter()
                .filter_map(|m| alive_q.get(*m).ok().map(|xp| (*m, xp.rank())))
                .max_by_key(|(_, r)| *r)
                .map(|(m, _)| m)
            {
                leader.0 = new;
            }
        }
    }
}
