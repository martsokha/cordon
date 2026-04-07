//! NPC attributes, roles, and employment.
//!
//! [`Npc`] is a runtime entity with both visible and hidden attributes.

use serde::{Deserialize, Serialize};

use super::faction::Faction;
use super::name::NpcName;
use super::perk::Perk;
use crate::item::Loadout;
use crate::primitive::{Credits, Experience, Health, Id, IdMarker, Rank, Uid};

/// Marker for NPC template IDs (used in quest consequences).
pub struct NpcTemplate;
impl IdMarker for NpcTemplate {}

/// What role an employed NPC fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Goes into the Zone to scavenge, deliver, or gather intel.
    Runner,
    /// Stays at the bunker to deter theft, enable intimidation, and fight raids.
    Guard,
}

/// Core personality trait affecting negotiation behavior (hidden).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Personality {
    /// Careful, slow to trust, thorough negotiator.
    Cautious,
    /// Confrontational, may escalate if refused.
    Aggressive,
    /// Straightforward, unlikely to scam.
    Honest,
    /// May lie about item quality or their situation.
    Deceptive,
    /// Willing to go back and forth on price.
    Patient,
    /// Makes snap decisions, may accept bad deals.
    Impulsive,
}

/// A non-player character in the game world.
///
/// NPCs have visible attributes (name, faction, rank, gear, condition)
/// and hidden attributes (trust, wealth, need, personality, perks).
/// Hidden attributes are never shown directly — the player infers them
/// through behavior over multiple interactions.
///
/// There is no explicit "NPC type" field — what an NPC is doing is
/// determined by the sim layer from their faction, rank, need, and
/// other attributes. A Drifter with high trust might be offered a job;
/// an Order Officer with a demand is a faction rep; a wounded NPC with
/// no credits is a desperate visitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    /// Unique runtime ID for this NPC instance.
    pub id: Uid<Npc>,
    /// Name stored as localization keys, resolved at display time.
    pub name: NpcName,
    /// Faction ID this NPC belongs to.
    pub faction: Id<Faction>,
    /// Accumulated experience. [`Rank`] is derived from this.
    pub xp: Experience,

    /// Equipped weapons, armor, and carried items.
    pub loadout: Loadout,
    /// Health. Drops from combat, radiation, hazards.
    pub health: Health,
    /// Maximum HP cap. Default 100; relics may modify it later.
    pub max_hp: u32,

    /// How much this NPC trusts the player (-1.0 to 1.0).
    pub trust: f32,
    /// How many credits the NPC can spend.
    pub wealth: Credits,
    /// Core personality trait affecting negotiation.
    pub personality: Personality,
    /// Perk IDs this NPC has (hidden until revealed).
    pub perks: Vec<Id<Perk>>,
    /// Perk IDs the player has discovered through gameplay.
    pub revealed_perks: Vec<Id<Perk>>,

    /// Current role if employed, or `None` if not hired.
    pub role: Option<Role>,
    /// Loyalty level (0.0–1.0). Drops with underpayment or suicide missions.
    pub loyalty: f32,
    /// How many credits this NPC expects per day.
    pub daily_pay: Credits,
}

impl Npc {
    /// Current rank, derived from XP.
    pub fn rank(&self) -> Rank {
        self.xp.npc_rank()
    }

    /// Whether this NPC is currently employed (has a role).
    pub fn is_employed(&self) -> bool {
        self.role.is_some()
    }

    /// Whether this NPC has a specific perk (by ID), even if unrevealed.
    pub fn has_perk(&self, perk_id: &Id<Perk>) -> bool {
        self.perks.iter().any(|p| p == perk_id)
    }

    /// Mark a perk as revealed to the player.
    ///
    /// Does nothing if the NPC doesn't have the perk or it's already revealed.
    pub fn reveal_perk(&mut self, perk_id: &Id<Perk>) {
        if self.has_perk(perk_id) && !self.revealed_perks.iter().any(|p| p == perk_id) {
            self.revealed_perks.push(perk_id.clone());
        }
    }
}
