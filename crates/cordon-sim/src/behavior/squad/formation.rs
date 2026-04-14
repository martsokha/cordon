//! Squad cohesion: formation slot positioning + the data that
//! anchors it.
//!
//! This module owns the three components that describe where the
//! squad is and where it's pointing — [`SquadFacing`],
//! [`SquadWaypoints`], [`SquadHomePosition`] — plus the
//! [`drive_squad_formation`] system that reads
//! [`MovementIntent`](super::intent::MovementIntent) and
//! [`EngagementTarget`](super::intent::EngagementTarget) and
//! writes per-member [`MovementTarget`] / [`MovementSpeed`].
//!
//! Throttled to ~10Hz. For each squad this:
//!
//! 1. Snapshots the leader position.
//! 2. Updates facing toward `MovementIntent` target when one is set.
//! 3. Per member, writes `MovementTarget`/`MovementSpeed` to either
//!    a combat target (if engaging and out of weapon range) or to the
//!    formation slot in the squad's local frame.
//!
//! All control-flow decisions (Protect follow, arrival detection,
//! hold transitions, goal-driven walking) live in the behavior tree
//! (`super::behave`), which writes [`MovementIntent`]; the scanner
//! ([`super::engagement`]) writes [`EngagementTarget`]. This module
//! is pure data flow: intent in, per-member movement out.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::squad::Formation;
use cordon_core::item::Loadout;
use cordon_data::gamedata::GameDataResource;

use super::constants::{
    ARRIVED_DIST, ENGAGE_WALK_SPEED, FORMATION_INTERVAL_SECS, SQUAD_WALK_SPEED,
};
use super::identity::{SquadLeader, SquadMembers, SquadMembership};
use super::intent::{EngagementTarget, MovementIntent};
use crate::behavior::combat::components::CombatTarget;
use crate::behavior::combat::helpers::weapon_range;
use crate::behavior::death::components::Dead;
use crate::behavior::movement::components::{MovementSpeed, MovementTarget};
use crate::entity::npc::NpcMarker;

/// Last known facing direction for formation rotation. Default is
/// +Y. Updated by [`drive_squad_formation`] to point toward the
/// current `MovementIntent` target so formation slots rotate with
/// the squad's direction of travel.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadFacing(pub Vec2);

impl Default for SquadFacing {
    fn default() -> Self {
        Self(Vec2::Y)
    }
}

/// Patrol/scavenge waypoints inside the goal area + the index of
/// the next one to visit. Consumed one-at-a-time by the
/// `BtWalkWaypoint` BT leaf. Empty for non-patrol goals.
#[derive(Component, Debug, Clone, Default)]
pub struct SquadWaypoints {
    pub points: Vec<Vec2>,
    pub next: u8,
}

/// Initial spawn position for the squad, used by the visual layer
/// to place freshly-spawned members at the right map coordinate
/// before the formation system takes over.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadHomePosition(pub Vec2);

/// Per-squad snapshot cached between the leader-pos pass and the
/// member-writing pass so we don't re-query mid-iteration.
#[derive(Clone, Copy)]
pub(super) struct SquadSnap {
    centroid: Vec2,
    facing: Vec2,
    formation: Formation,
    member_count: usize,
    engaged: Option<Entity>,
}

pub(super) fn drive_squad_formation(
    game_data: Res<GameDataResource>,
    time: Res<Time>,
    mut throttle: Local<f32>,
    mut squad_leader_pos: Local<HashMap<Entity, Vec2>>,
    mut squad_info: Local<HashMap<Entity, SquadSnap>>,
    mut squad_state_q: Query<(
        Entity,
        &SquadLeader,
        &SquadMembers,
        &Formation,
        &MovementIntent,
        &EngagementTarget,
        &mut SquadFacing,
    )>,
    leaders_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut members_q: Query<
        (
            &SquadMembership,
            &Transform,
            &CombatTarget,
            &Loadout,
            &mut MovementTarget,
            &mut MovementSpeed,
        ),
        Without<Dead>,
    >,
    targets_q: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
) {
    *throttle += time.delta_secs();
    if *throttle < FORMATION_INTERVAL_SECS {
        return;
    }
    *throttle = 0.0;

    let items = &game_data.0.items;

    squad_leader_pos.clear();
    for (squad_entity, leader, _, _, _, _, _) in squad_state_q.iter() {
        if let Ok(t) = leaders_q.get(leader.0) {
            squad_leader_pos.insert(squad_entity, t.translation.truncate());
        }
    }

    // Pass A: per-squad — update facing toward the movement target
    // when one is set, so formation slots rotate into the direction
    // of travel. Arrival, Protect follow, and Hold transitions are
    // handled by the behavior tree and surface here as `MovementIntent`.
    for (squad_entity, _, _, _, intent, _, mut facing) in squad_state_q.iter_mut() {
        let Some(p) = squad_leader_pos.get(&squad_entity).copied() else {
            continue;
        };
        if let Some(target) = intent.0 {
            let new_facing = (target - p).normalize_or_zero();
            if new_facing.length_squared() > 0.001 {
                facing.0 = new_facing;
            }
        }
        if facing.0 == Vec2::ZERO {
            facing.0 = Vec2::Y;
        }
    }

    squad_info.clear();
    for (e, _, members, formation, intent, engagement, facing) in squad_state_q.iter() {
        let leader_p = squad_leader_pos.get(&e).copied().unwrap_or(Vec2::ZERO);
        let centroid = intent.0.unwrap_or(leader_p);
        squad_info.insert(
            e,
            SquadSnap {
                centroid,
                facing: facing.0,
                formation: *formation,
                member_count: members.0.len(),
                engaged: engagement.0,
            },
        );
    }

    for (member, transform, combat_target, loadout, mut move_target, mut speed) in &mut members_q {
        let Some(snap) = squad_info.get(&member.squad).copied() else {
            continue;
        };
        let pos = transform.translation.truncate();

        if snap.engaged.is_some() {
            if let Some(target_entity) = combat_target.0
                && let Ok(target_t) = targets_q.get(target_entity)
            {
                let target_pos = target_t.translation.truncate();
                let dist = pos.distance(target_pos);
                let range = weapon_range(items, loadout);
                if range > 0.0 && dist <= range {
                    if move_target.0.is_some() {
                        move_target.0 = None;
                    }
                } else {
                    set_movement_target(
                        &mut move_target,
                        &mut speed,
                        target_pos,
                        ENGAGE_WALK_SPEED,
                    );
                }
            } else {
                // Squad is engaged but this member has no specific
                // target — either they have no LOS to any hostile or
                // their assigned target was just despawned. Regroup
                // on the leader so they reposition into formation
                // and have a chance to regain LOS, instead of
                // standing frozen in place.
                if let Some(leader_p) = squad_leader_pos.get(&member.squad).copied() {
                    if pos.distance(leader_p) > ARRIVED_DIST {
                        set_movement_target(
                            &mut move_target,
                            &mut speed,
                            leader_p,
                            ENGAGE_WALK_SPEED,
                        );
                    } else if move_target.0.is_some() {
                        move_target.0 = None;
                    }
                } else if move_target.0.is_some() {
                    move_target.0 = None;
                }
            }
            continue;
        }

        let facing = snap.facing.normalize_or_zero();
        if facing.length_squared() < 0.001 {
            continue;
        }
        let offsets = snap.formation.slot_offsets(snap.member_count);
        let slot = (member.slot as usize).min(offsets.len().saturating_sub(1));
        let local = Vec2::new(offsets[slot][0], offsets[slot][1]);
        let perp = Vec2::new(-facing.y, facing.x);
        let world_offset = perp * local.x + facing * local.y;
        let target = snap.centroid + world_offset;

        let dist = pos.distance(target);
        if dist > ARRIVED_DIST {
            set_movement_target(&mut move_target, &mut speed, target, SQUAD_WALK_SPEED);
        } else if move_target.0.is_some() {
            move_target.0 = None;
        }
    }
}

fn set_movement_target(
    move_target: &mut MovementTarget,
    speed: &mut MovementSpeed,
    new_target: Vec2,
    new_speed: f32,
) {
    let same = move_target
        .0
        .map(|t| t.distance(new_target) < 1.0)
        .unwrap_or(false);
    if !same {
        move_target.0 = Some(new_target);
    }
    if (speed.0 - new_speed).abs() > 0.5 {
        speed.0 = new_speed;
    }
}
