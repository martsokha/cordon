//! Squads — coordinated groups of NPCs with a shared goal.
//!
//! [`Squad`] is the **spawn-time / save-game** representation. At
//! runtime, squads are Bevy entities composed of components defined
//! in `cordon-sim::components` (`SquadFaction`, `SquadGoal`,
//! `SquadFormation`, `SquadFacing`, `SquadWaypoints`, `SquadLeader`,
//! `SquadMembers`, `SquadActivity`).
//!
//! Vision and engagement are shared at squad granularity: when any
//! member spots a hostile, the whole squad knows; when the squad
//! commits to engaging, every member targets that hostile *squad*
//! (each picking their own nearest enemy from it).

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use super::faction::Faction;
use super::npc::Npc;
use crate::primitive::{Id, IdMarker, Uid};

/// A coordinated group of NPCs sharing a goal and (when fighting) a
/// target squad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Squad {
    /// Unique runtime ID for this squad.
    pub id: Uid<Squad>,
    /// Faction this squad belongs to. All members share it.
    pub faction: Id<Faction>,
    /// All member NPC uids (alive or dead). Cleanup removes the squad
    /// once everyone is dead and despawned.
    pub members: Vec<Uid<Npc>>,
    /// Current leader's uid. Always one of [`members`](Squad::members);
    /// promoted to the highest-rank survivor when the previous leader dies.
    pub leader: Uid<Npc>,
    /// Long-term reason this squad exists. Survives leader death.
    pub goal: Goal,
    /// Formation the squad walks in when not fighting.
    pub formation: Formation,
    /// Last non-zero direction the squad was facing. Drives formation
    /// rotation; updated by movement, by enemy sightings, or kept stable.
    pub facing: [f32; 2],
    /// Patrol waypoints for Patrol/Scavenge goals: a small list of
    /// world-space points the squad cycles through, holding briefly at
    /// each. Empty for non-patrol goals.
    pub waypoints: Vec<[f32; 2]>,
    /// Index of the next waypoint to head to.
    pub next_waypoint: u8,
}

/// Long-term reason a squad exists. Survives leader death and is the
/// only "memory" the squad has between activities.
#[derive(Debug, Clone)]
#[derive(Component, Serialize, Deserialize)]
pub enum Goal {
    /// No orders. Hold position or wander loosely.
    Idle,
    /// Cycle through waypoints inside an area, holding briefly at each.
    Patrol { area: Id<crate::world::area::Area> },
    /// Move into an area and look for loot until carrying capacity is full.
    Scavenge { area: Id<crate::world::area::Area> },
    /// Stay close to another squad and engage anyone who attacks them.
    Protect { other: Uid<Squad> },
    /// Locate a target NPC and engage on contact.
    Find { target: Uid<Npc> },
    /// Carry items to a destination point.
    Deliver { to: [f32; 2] },
}

/// Formation the squad walks in when not in combat.
///
/// Combat overrides formation choice (members spread out or focus on
/// targets), but the squad reverts to its formation when combat ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Component, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formation {
    /// Members in a row perpendicular to facing.
    Line,
    /// Leader at point, others trailing in a V.
    Wedge,
    /// Single file behind the leader.
    Column,
    /// Random scatter within a small radius around the leader.
    Loose,
}

impl Formation {
    /// Slot offsets relative to the leader, in the leader's local frame
    /// (forward = +Y). Returns one offset per slot index. Slot 0 is the
    /// leader (always at the origin). Capped at 5 members.
    pub fn slot_offsets(self, n_members: usize) -> Vec<[f32; 2]> {
        let n = n_members.min(5);
        let mut out = Vec::with_capacity(n);
        // Leader is always slot 0 at the origin.
        out.push([0.0, 0.0]);
        if n <= 1 {
            return out;
        }
        // Spacing between members (map units).
        const SPACING: f32 = 22.0;
        match self {
            Formation::Line => {
                // Row perpendicular to facing: members at ±SPACING along x.
                // 2 members → [+S, 0]; 3 → [-S, +S, 0]; 4 → [-1.5S, -.5S, +.5S, +1.5S].
                // Slot 0 is the leader, so we fill slots 1..n with offsets.
                for i in 1..n {
                    let pair = i.div_ceil(2) as f32;
                    let sign = if i % 2 == 1 { 1.0 } else { -1.0 };
                    out.push([sign * pair * SPACING, 0.0]);
                }
            }
            Formation::Wedge => {
                // Leader at point, members trailing in a V (-Y).
                for i in 1..n {
                    let depth = i.div_ceil(2) as f32;
                    let sign = if i % 2 == 1 { 1.0 } else { -1.0 };
                    out.push([sign * depth * SPACING * 0.7, -depth * SPACING]);
                }
            }
            Formation::Column => {
                // Single file: each member directly behind the previous.
                for i in 1..n {
                    out.push([0.0, -(i as f32) * SPACING]);
                }
            }
            Formation::Loose => {
                // Cheap deterministic scatter from a small fixed table.
                const SCATTER: [[f32; 2]; 4] =
                    [[-12.0, -8.0], [10.0, -10.0], [-9.0, 11.0], [13.0, 6.0]];
                for i in 1..n {
                    out.push(SCATTER[(i - 1) % 4]);
                }
            }
        }
        out
    }
}

impl IdMarker for Squad {}
