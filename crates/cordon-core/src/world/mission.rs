//! Runner mission types, plans, and results.
//!
//! Missions are dispatched during the Morning phase. Runners travel
//! to their destination, complete the mission, and return. Travel
//! time is computed dynamically by the sim — runners can be tracked
//! in real-time on the laptop map.

use serde::{Deserialize, Serialize};

use super::area::Area;
use crate::entity::npc::Npc;
use crate::item::{ItemCategory, ItemInstance};
use crate::primitive::{Day, Id, Location, Uid};

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
        items: Vec<ItemInstance>,
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
    pub id: Uid<MissionPlan>,
    /// Runtime UID of the runner being sent.
    pub runner_id: Uid<Npc>,
    /// Destination area ID.
    pub destination: Id<Area>,
    /// What kind of mission this is.
    pub mission_type: MissionType,
}

/// A mission that has been dispatched and is currently in progress.
///
/// The runner is traveling to the destination, completing the mission,
/// and returning. The sim computes the return day dynamically based
/// on distance and hazards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMission {
    /// The original mission plan.
    pub plan: MissionPlan,
    /// Day the mission was dispatched.
    pub day_dispatched: Day,
    /// Day the runner is expected to return. Computed by the sim
    /// at dispatch time based on sector distance and events.
    pub return_day: Day,
    /// Current position on the map, updated each tick by the sim.
    pub current_location: Location,
}

/// The result of a completed mission, returned to the game layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// ID of the mission that completed.
    pub mission_id: Uid<MissionPlan>,
    /// What happened.
    pub outcome: MissionOutcome,
    /// Items the runner brought back (empty on failure/lost).
    pub loot: Vec<ItemInstance>,
    /// HP damage taken by the runner during the mission (0 = unscathed).
    pub runner_damage: u32,
    /// Durability damage taken by the runner's gear during the mission.
    pub gear_damage: u32,
}
