//! Squad identity: who this squad is and who belongs to it.
//!
//! Pure ECS data — no systems. The marker, leader pointer, member
//! list, and the back-pointer each NPC carries to its squad entity.
//! [`SquadBundle`] composes all of them plus the cohesion-side
//! components (facing / waypoints / home, owned by
//! [`super::formation`]) and the blackboard intent components
//! (owned by [`super::intent`]) into one spawn bundle.
//!
//! Lifecycle systems that prune stale members and promote new
//! leaders live in [`super::lifecycle`].

use bevy::prelude::*;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::primitive::Uid;

use super::formation::{SquadFacing, SquadHomePosition, SquadWaypoints};
use super::intent::{EngagementTarget, MovementIntent};
use crate::entity::npc::FactionId;

/// Marker that this entity is a squad.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMarker;

/// The current leader's entity. Promoted to highest-rank
/// survivor when the previous leader dies.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadLeader(pub Entity);

/// All member entities (alive). Pruned by `cleanup_dead_squads`.
#[derive(Component, Debug, Clone)]
pub struct SquadMembers(pub Vec<Entity>);

/// Back-pointer from an NPC entity to its squad entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMembership {
    pub squad: Entity,
    /// Formation slot index (0 = leader, 1..=4 = followers).
    pub slot: u8,
}

#[derive(Bundle)]
pub struct SquadBundle {
    pub marker: SquadMarker,
    pub id: Uid<Squad>,
    pub faction: FactionId,
    pub goal: Goal,
    pub formation: Formation,
    pub facing: SquadFacing,
    pub waypoints: SquadWaypoints,
    pub home: SquadHomePosition,
    pub leader: SquadLeader,
    pub members: SquadMembers,
    pub movement: MovementIntent,
    pub engagement: EngagementTarget,
}

impl SquadBundle {
    pub fn from_squad(squad: Squad, leader: Entity, members: Vec<Entity>, home: Vec2) -> Self {
        Self {
            marker: SquadMarker,
            id: squad.id,
            faction: FactionId(squad.faction),
            goal: squad.goal,
            formation: squad.formation,
            facing: SquadFacing(Vec2::new(squad.facing[0], squad.facing[1])),
            waypoints: SquadWaypoints {
                points: squad
                    .waypoints
                    .into_iter()
                    .map(|p| Vec2::new(p[0], p[1]))
                    .collect(),
                next: squad.next_waypoint,
            },
            home: SquadHomePosition(home),
            leader: SquadLeader(leader),
            members: SquadMembers(members),
            movement: MovementIntent::default(),
            engagement: EngagementTarget::default(),
        }
    }
}
