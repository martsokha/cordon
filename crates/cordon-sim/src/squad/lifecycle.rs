//! Squad lifecycle: drop dead members, promote new leaders, despawn
//! fully-dead squads. Throttled to 1 Hz.

use bevy::prelude::*;

use crate::behavior::Dead;
use crate::components::{NpcMarker, SquadLeader, SquadMembers, Xp};

pub(super) fn cleanup_dead_squads(
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut commands: Commands,
    alive_q: Query<&Xp, (With<NpcMarker>, Without<Dead>)>,
    mut squads_q: Query<(Entity, &mut SquadMembers, &mut SquadLeader)>,
) {
    const CLEANUP_INTERVAL_SECS: f32 = 1.0;
    *throttle += time.delta_secs();
    if *throttle < CLEANUP_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    for (squad_entity, mut members, mut leader) in &mut squads_q {
        members.0.retain(|m| alive_q.get(*m).is_ok());
        if members.0.is_empty() {
            commands.entity(squad_entity).despawn();
            continue;
        }
        if alive_q.get(leader.0).is_err() {
            let new_leader = members
                .0
                .iter()
                .filter_map(|m| alive_q.get(*m).ok().map(|xp| (*m, xp.rank())))
                .max_by_key(|(_, r)| *r)
                .map(|(m, _)| m);
            if let Some(new) = new_leader {
                leader.0 = new;
            } else {
                commands.entity(squad_entity).despawn();
            }
        }
    }
}
