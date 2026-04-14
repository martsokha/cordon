//! Per-squad ECS components.
//!
//! `Goal` and `Formation` derive `Component` directly in
//! cordon-core, so they're attached to squad entities without a
//! wrapper. This module holds the squad marker, the position /
//! facing / waypoints live state, leader + member entity lists,
//! the short-term activity state machine, and the `SquadBundle`
//! glue.

use bevy::prelude::*;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::primitive::Uid;

use super::npc::FactionId;

/// Marker that this entity is a squad.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMarker;

/// Last known facing direction for formation rotation. Default
/// is +Y.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadFacing(pub Vec2);

impl Default for SquadFacing {
    fn default() -> Self {
        Self(Vec2::Y)
    }
}

/// Patrol/scavenge waypoints inside the goal area + the index
/// of the next one to visit. Empty for non-patrol goals.
#[derive(Component, Debug, Clone, Default)]
pub struct SquadWaypoints {
    pub points: Vec<Vec2>,
    pub next: u8,
}

/// Initial spawn position for the squad, used by the visual
/// layer to place freshly-spawned members at the right map
/// coordinate before the formation system takes over.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadHomePosition(pub Vec2);

/// The current leader's entity. Promoted to highest-rank
/// survivor when the previous leader dies.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadLeader(pub Entity);

/// All member entities (alive). Pruned by `cleanup_dead_squads`.
#[derive(Component, Debug, Clone)]
pub struct SquadMembers(pub Vec<Entity>);

/// Where the squad wants its leader-anchored centroid to be this
/// frame. `Some(target)` = walk there; `None` = hold position (use
/// the leader's current transform as the centroid). Written by
/// behavior-tree action leaves (see `squad/behave`); read by
/// `drive_squad_formation` to place members in their formation slot.
///
/// Replaces the old `SquadActivity::Move` variant. Absence replaces
/// `SquadActivity::Hold`.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovementIntent(pub Option<Vec2>);

/// Hostile squad entity the engagement scanner has locked onto this
/// frame, if any. Written by `update_squad_engagement`; read by the
/// formation per-member pass (to chase a combat target when out of
/// weapon range) and by behavior trees (for branching on "we're
/// currently engaging").
///
/// Replaces the old `SquadActivity::Engage` variant.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EngagementTarget(pub Option<Entity>);

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

/// Back-pointer from an NPC entity to its squad entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMembership {
    pub squad: Entity,
    /// Formation slot index (0 = leader, 1..=4 = followers).
    pub slot: u8,
}
