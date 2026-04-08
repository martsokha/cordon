//! Per-squad ECS components.
//!
//! `Goal` and `Formation` derive `Component` directly in
//! cordon-core, so they're attached to squad entities without a
//! wrapper. This module holds the squad marker, the position /
//! facing / waypoints live state, leader + member entity lists,
//! the short-term activity state machine, and the `SquadBundle`
//! glue.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::primitive::{Id, Uid};

/// Marker that this entity is a squad.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMarker;

/// Faction membership. Same "distinct from `Id<Faction>` in
/// data-struct fields" reasoning as `FactionId` on NPCs.
#[derive(Component, Debug, Clone)]
pub struct SquadFaction(pub Id<Faction>);

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

/// Short-term squad activity (Hold / Move / Engage). The squad
/// systems read and write this each tick.
#[derive(Component, Debug, Clone)]
pub enum SquadActivity {
    Hold { duration_secs: f32 },
    Move { target: Vec2 },
    Engage { hostiles: Entity },
}

impl Default for SquadActivity {
    fn default() -> Self {
        Self::Hold { duration_secs: 1.0 }
    }
}

#[derive(Bundle)]
pub struct SquadBundle {
    pub marker: SquadMarker,
    pub id: Uid<Squad>,
    pub faction: SquadFaction,
    pub goal: Goal,
    pub formation: Formation,
    pub facing: SquadFacing,
    pub waypoints: SquadWaypoints,
    pub home: SquadHomePosition,
    pub leader: SquadLeader,
    pub members: SquadMembers,
    pub activity: SquadActivity,
}

impl SquadBundle {
    pub fn from_squad(squad: Squad, leader: Entity, members: Vec<Entity>, home: Vec2) -> Self {
        Self {
            marker: SquadMarker,
            id: squad.id,
            faction: SquadFaction(squad.faction),
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
            activity: SquadActivity::default(),
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
