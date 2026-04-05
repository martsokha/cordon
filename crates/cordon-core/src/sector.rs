use serde::{Deserialize, Serialize};

use crate::faction::FactionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectorId {
    Threshold,
    Scrapyard,
    Hollows,
    Crossroads,
    Tangles,
    Depot,
    DeepWoods,
    Core,
}

impl SectorId {
    pub const ALL: [SectorId; 8] = [
        SectorId::Threshold,
        SectorId::Scrapyard,
        SectorId::Hollows,
        SectorId::Crossroads,
        SectorId::Tangles,
        SectorId::Depot,
        SectorId::DeepWoods,
        SectorId::Core,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SectorId::Threshold => "The Threshold",
            SectorId::Scrapyard => "The Scrapyard",
            SectorId::Hollows => "The Hollows",
            SectorId::Crossroads => "The Crossroads",
            SectorId::Tangles => "The Tangles",
            SectorId::Depot => "The Depot",
            SectorId::DeepWoods => "The Deep Woods",
            SectorId::Core => "The Core",
        }
    }

    pub fn radio_level_required(self) -> u8 {
        match self {
            SectorId::Threshold | SectorId::Scrapyard => 1,
            SectorId::Hollows | SectorId::Crossroads => 2,
            SectorId::Tangles | SectorId::Depot => 3,
            SectorId::DeepWoods => 4,
            SectorId::Core => 5,
        }
    }

    pub fn base_danger(self) -> f32 {
        match self {
            SectorId::Threshold => 0.1,
            SectorId::Scrapyard => 0.25,
            SectorId::Hollows => 0.4,
            SectorId::Crossroads => 0.15,
            SectorId::Tangles => 0.5,
            SectorId::Depot => 0.65,
            SectorId::DeepWoods => 0.8,
            SectorId::Core => 0.95,
        }
    }

    pub fn base_reward(self) -> f32 {
        match self {
            SectorId::Threshold => 0.1,
            SectorId::Scrapyard => 0.2,
            SectorId::Hollows => 0.4,
            SectorId::Crossroads => 0.35,
            SectorId::Tangles => 0.6,
            SectorId::Depot => 0.7,
            SectorId::DeepWoods => 0.85,
            SectorId::Core => 1.0,
        }
    }

    /// How many days a round trip takes.
    pub fn travel_days(self) -> u32 {
        match self {
            SectorId::Threshold | SectorId::Scrapyard | SectorId::Crossroads => 1,
            SectorId::Hollows | SectorId::Tangles | SectorId::Depot => 1,
            SectorId::DeepWoods => 2,
            SectorId::Core => 2,
        }
    }
}

/// Static sector definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorDef {
    pub id: SectorId,
    pub default_faction: Option<FactionId>,
}
