use serde::{Deserialize, Serialize};

use crate::faction::FactionId;
use crate::item::ItemStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NpcId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Tier1,
    Tier2,
    Tier3,
    Tier4,
    Tier5,
}

impl Rank {
    pub fn military_title(self) -> &'static str {
        match self {
            Rank::Tier1 => "Grunt",
            Rank::Tier2 => "Soldier",
            Rank::Tier3 => "Veteran",
            Rank::Tier4 => "Officer",
            Rank::Tier5 => "Commander",
        }
    }

    pub fn loose_title(self) -> &'static str {
        match self {
            Rank::Tier1 => "Rookie",
            Rank::Tier2 => "Seasoned",
            Rank::Tier3 => "Hardened",
            Rank::Tier4 => "Boss",
            Rank::Tier5 => "Legend",
        }
    }

    pub fn religious_title(self) -> &'static str {
        match self {
            Rank::Tier1 => "Pilgrim",
            Rank::Tier2 => "Acolyte",
            Rank::Tier3 => "Keeper",
            Rank::Tier4 => "Prophet",
            Rank::Tier5 => "Ascended",
        }
    }

    pub fn title_for(self, faction: FactionId) -> &'static str {
        match faction {
            FactionId::Order | FactionId::Garrison => self.military_title(),
            FactionId::Devoted => self.religious_title(),
            _ => self.loose_title(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Perk {
    ScavengersEye,
    HardToKill,
    Pathfinder,
    Haggler,
    Ghost,
    Ironwall,
    Intimidating,
    StickyFingers,
    Coward,
    BigMouth,
    GrudgeHolder,
    Lucky,
}

impl Perk {
    pub fn is_positive(self) -> bool {
        matches!(
            self,
            Perk::ScavengersEye
                | Perk::HardToKill
                | Perk::Pathfinder
                | Perk::Haggler
                | Perk::Ghost
                | Perk::Ironwall
                | Perk::Intimidating
        )
    }

    pub fn is_negative(self) -> bool {
        matches!(
            self,
            Perk::StickyFingers | Perk::Coward | Perk::BigMouth | Perk::GrudgeHolder
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Personality {
    Cautious,
    Aggressive,
    Honest,
    Deceptive,
    Patient,
    Impulsive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Need {
    None,
    Wounded,
    Starving,
    Desperate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpcType {
    Drifter,
    FactionSoldier,
    JobSeeker,
    FactionRep,
    Scammer,
    DesperateVisitor,
    Informant,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Runner,
    Guard,
}

/// Condition of an NPC (visible to the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpcCondition {
    Healthy,
    Wounded,
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    pub id: NpcId,
    pub name: String,
    pub faction: FactionId,
    pub rank: Rank,
    pub npc_type: NpcType,

    // Visible
    pub gear: Vec<ItemStack>,
    pub condition: NpcCondition,

    // Hidden
    pub trust: f32,
    pub wealth: u32,
    pub need: Need,
    pub personality: Personality,
    pub perks: Vec<Perk>,
    pub revealed_perks: Vec<Perk>,

    // Employment
    pub role: Option<Role>,
    pub loyalty: f32,
    pub daily_pay: u32,
}

impl Npc {
    pub fn is_employed(&self) -> bool {
        self.role.is_some()
    }

    pub fn has_perk(&self, perk: Perk) -> bool {
        self.perks.contains(&perk)
    }

    pub fn reveal_perk(&mut self, perk: Perk) {
        if self.perks.contains(&perk) && !self.revealed_perks.contains(&perk) {
            self.revealed_perks.push(perk);
        }
    }
}
