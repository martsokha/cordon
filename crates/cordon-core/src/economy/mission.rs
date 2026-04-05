use serde::{Deserialize, Serialize};

use crate::economy::item::{ItemKind, ItemStack};
use crate::entity::npc::{NpcId, Perk};
use crate::world::sector::SectorId;
use crate::world::time::Day;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MissionId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissionType {
    Scavenge,
    TargetedSearch(ItemKind),
    Delivery { items: Vec<ItemStack>, to: SectorId },
    Recon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissionOutcome {
    Success,
    PartialSuccess,
    Failure,
    RunnerLost,
    Jackpot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    pub id: MissionId,
    pub runner_id: NpcId,
    pub destination: SectorId,
    pub mission_type: MissionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveMission {
    pub plan: MissionPlan,
    pub day_dispatched: Day,
    pub return_day: Day,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    pub mission_id: MissionId,
    pub outcome: MissionOutcome,
    pub loot: Vec<ItemStack>,
    pub runner_condition_delta: f32,
    pub gear_condition_delta: f32,
    pub perks_revealed: Vec<Perk>,
}
