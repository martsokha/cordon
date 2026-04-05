//! NPC attributes, perks, and employment.
//!
//! [`PerkDef`] is loaded from config. [`Npc`] is a runtime entity
//! with both visible and hidden attributes.

use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::primitive::id::{Id, Faction, Perk};
use crate::primitive::uid::Uid;

/// Polarity of a perk's effect on the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerkPolarity {
    /// Beneficial to the player (e.g., Scavenger's Eye, Hard to Kill).
    Positive,
    /// Harmful to the player (e.g., Coward, Sticky Fingers).
    Negative,
    /// Unpredictable or mixed (e.g., Lucky).
    Neutral,
}

/// Perk definition loaded from config.
///
/// Perks are hidden NPC traits revealed through gameplay actions.
/// Each perk has a unique ID and affects runner missions or guard duty.
/// The [`id`](PerkDef::id) doubles as the localization key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerkDef {
    /// Unique identifier and localization key (e.g., `"scavengers_eye"`).
    pub id: Id<Perk>,
    /// Whether this perk helps, hurts, or is unpredictable.
    pub polarity: PerkPolarity,
}

/// What role an employed NPC fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Goes into the Zone to scavenge, deliver, or gather intel.
    Runner,
    /// Stays at the bunker to deter theft, enable intimidation, and fight raids.
    Guard,
}

/// Physical condition of an NPC (visible to the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NpcCondition {
    /// Fully functional.
    Healthy,
    /// Reduced performance, needs medical attention.
    Wounded,
    /// Reduced performance from overwork or stress.
    Exhausted,
}

/// What the NPC urgently needs (hidden from the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Need {
    /// No urgent need.
    None,
    /// Needs medical supplies.
    Wounded,
    /// Needs food.
    Starving,
    /// Will accept almost any deal.
    Desperate,
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
    pub id: Uid,
    /// Display name or alias (e.g., "Viper", "Matches").
    pub name: String,
    /// Faction ID this NPC belongs to.
    pub faction: Id<Faction>,
    /// Rank tier (1–5). Title comes from the faction's config.
    pub rank: u8,

    // Visible
    /// Items the NPC is carrying.
    pub gear: Vec<Item>,
    /// Physical condition (visible from appearance).
    pub condition: NpcCondition,
    /// How many inventory slots this NPC can carry.
    pub inventory_slots: u8,

    // Hidden
    /// How much this NPC trusts the player (-1.0 to 1.0).
    pub trust: f32,
    /// How many credits the NPC can spend.
    pub wealth: u32,
    /// What the NPC actually needs (may differ from what they say).
    pub need: Need,
    /// Core personality trait affecting negotiation.
    pub personality: Personality,
    /// Perk IDs this NPC has (hidden until revealed).
    pub perks: Vec<Id<Perk>>,
    /// Perk IDs the player has discovered through gameplay.
    pub revealed_perks: Vec<Id<Perk>>,

    // Employment
    /// Current role if employed, or `None` if not hired.
    pub role: Option<Role>,
    /// Loyalty level (0.0–1.0). Drops with underpayment or suicide missions.
    pub loyalty: f32,
    /// How many credits this NPC expects per day.
    pub daily_pay: u32,
}

impl Npc {
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
