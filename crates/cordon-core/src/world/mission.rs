//! Runner mission types, plans, and results.
//!
//! Missions are dispatched during the Morning phase. Runners travel
//! to their destination, complete the mission, and return. Travel
//! time is computed dynamically by the sim — runners can be tracked
//! in real-time on the laptop map.

use serde::{Deserialize, Serialize};

use crate::entity::perk::Perk;
use crate::item::{Item, ItemCategory};
use crate::primitive::id::Id;
use crate::primitive::location::Location;
use crate::primitive::time::Day;
use crate::primitive::uid::Uid;
use crate::world::area::Area;

/// What kind of mission a runner is being sent on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionType {
    /// Bring back whatever they find.
    Scavenge,
    /// Look for a specific item category (costs more, less total loot).
    TargetedSearch(ItemCategory),
    /// Bring goods to a buyer in another sector (guaranteed sale, transit risk).
    Delivery {
        /// Items being delivered.
        items: Vec<Item>,
        /// Destination area ID.
        to: Id<Area>,
    },
    /// Gather intel on a sector (no loot, but information).
    Recon,
}

/// The outcome of a completed mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionOutcome {
    /// Full loot, runner returns healthy.
    Success,
    /// Some loot, runner may be wounded.
    PartialSuccess,
    /// No loot, runner wounded or lost gear.
    Failure,
    /// Runner doesn't return. Presumed dead. Gear lost.
    RunnerLost,
    /// Exceptional find: rare relic, stash, or intel.
    Jackpot,
}

/// A mission plan before dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    /// Unique mission ID.
    pub id: Uid,
    /// Runtime UID of the runner being sent.
    pub runner_id: Uid,
    /// Destination area ID.
    pub destination: Id<Area>,
    /// What kind of mission this is.
    pub mission_type: MissionType,
}

/// A mission that has been dispatched and is currently in progress.
///
/// The runner is traveling to the destination, completing the mission,
/// and returning. The sim computes the return day dynamically based
/// on distance, hazards, and runner perks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMission {
    /// The original mission plan.
    pub plan: MissionPlan,
    /// Day the mission was dispatched.
    pub day_dispatched: Day,
    /// Day the runner is expected to return. Computed by the sim
    /// at dispatch time based on sector distance, events, and perks.
    pub return_day: Day,
    /// Current position on the map, updated each tick by the sim.
    pub current_location: Location,
}

/// The result of a completed mission, returned to the game layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// ID of the mission that completed.
    pub mission_id: Uid,
    /// What happened.
    pub outcome: MissionOutcome,
    /// Items the runner brought back (empty on failure/lost).
    pub loot: Vec<Item>,
    /// Change to the runner's condition (negative = damage).
    /// Applied via [`Condition::degrade()`](crate::primitive::condition::Condition::degrade).
    pub runner_condition_delta: f32,
    /// Change to the runner's gear condition (negative = wear).
    /// Applied via [`Condition::degrade()`](crate::primitive::condition::Condition::degrade).
    pub gear_condition_delta: f32,
    /// Perk IDs that were revealed by this mission's events.
    pub perks_revealed: Vec<Id<Perk>>,
}
