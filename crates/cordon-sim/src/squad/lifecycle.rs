//! Squad lifecycle: prune fully-despawned members from the squad's
//! member list, promote new leaders when the old one dies, and
//! despawn squads with zero remaining members. Throttled to 1 Hz.
//!
//! A squad whose members are all dead but whose corpses still exist
//! stays alive — the corpses are members for lifecycle purposes, and
//! the squad only despawns once the last corpse has been despawned by
//! the corpse cleanup path. This keeps dead bodies lootable for as
//! long as they exist on the map.

use bevy::prelude::*;

use crate::behavior::Dead;
use crate::components::{NpcMarker, SquadLeader, SquadMembers, Xp};
use crate::tuning::CLEANUP_INTERVAL_SECS;

pub(super) fn cleanup_dead_squads(
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut commands: Commands,
    alive_q: Query<&Xp, (With<NpcMarker>, Without<Dead>)>,
    member_exists_q: Query<(), With<NpcMarker>>,
    mut squads_q: Query<(Entity, &mut SquadMembers, &mut SquadLeader)>,
) {
    *throttle += time.delta_secs();
    if *throttle < CLEANUP_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    for (squad_entity, mut members, mut leader) in &mut squads_q {
        // Drop only members whose entity has been despawned. Corpses
        // still tagged with NpcMarker count as members so the squad
        // outlives them and stays lootable.
        members.0.retain(|m| member_exists_q.get(*m).is_ok());
        if members.0.is_empty() {
            commands.entity(squad_entity).despawn();
            continue;
        }
        if alive_q.get(leader.0).is_err() {
            // Promote the highest-rank surviving alive member. If no
            // members are alive (all corpses), leave the leader field
            // pointing at the dead one — engagement/formation systems
            // already gate on alive checks.
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
