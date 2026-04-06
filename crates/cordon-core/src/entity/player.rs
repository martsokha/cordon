//! Player state: rank, experience, credits, faction standings, and squads.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Faction;
use crate::entity::npc::{Npc, Role};
use crate::primitive::credits::Credits;
use crate::primitive::experience::Experience;
use crate::primitive::id::Id;
use crate::primitive::relation::Relation;
use crate::primitive::uid::Uid;

/// Player rank tier. Determines squad capacity and unlocks.
///
/// Rank is derived from accumulated [`Experience`] — the player ranks up
/// automatically when their XP crosses a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub enum PlayerRank {
    /// Starting rank. 2 squads.
    Nobody = 1,
    /// Built some reputation. 3 squads.
    Known = 2,
    /// Sustained trade volume, multiple faction relationships. 4 squads.
    Established = 3,
    /// High faction standings, major deals completed. 5 squads.
    Connected = 4,
    /// Endgame — Zone-wide reputation. 6 squads.
    Legend = 5,
}

impl PlayerRank {
    /// Maximum number of squads (runners + guards) at this rank.
    pub fn max_squads(self) -> u8 {
        match self {
            PlayerRank::Nobody => 1,
            PlayerRank::Known => 2,
            PlayerRank::Established => 3,
            PlayerRank::Connected => 4,
            PlayerRank::Legend => 5,
        }
    }

    /// The numeric tier (1–5).
    pub fn tier(self) -> u8 {
        self as u8
    }

    /// Minimum XP required to reach this rank.
    pub fn xp_threshold(self) -> u32 {
        match self {
            PlayerRank::Nobody => 0,
            PlayerRank::Known => 500,
            PlayerRank::Established => 2000,
            PlayerRank::Connected => 5000,
            PlayerRank::Legend => 15000,
        }
    }

    /// Determine rank from experience.
    pub fn from_xp(xp: Experience) -> Self {
        let v = xp.value();
        if v >= PlayerRank::Legend.xp_threshold() {
            PlayerRank::Legend
        } else if v >= PlayerRank::Connected.xp_threshold() {
            PlayerRank::Connected
        } else if v >= PlayerRank::Established.xp_threshold() {
            PlayerRank::Established
        } else if v >= PlayerRank::Known.xp_threshold() {
            PlayerRank::Known
        } else {
            PlayerRank::Nobody
        }
    }
}

/// A hired NPC assigned to a role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadMember {
    /// Runtime UID of the hired NPC.
    pub npc_id: Uid<Npc>,
    /// Whether this NPC is a runner or guard.
    pub role: Role,
}

/// The player's current state.
///
/// Tracks experience, credits, faction standings, and the squad roster.
/// Rank is derived from XP via [`PlayerRank::from_xp`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Accumulated experience. Rank is derived from this.
    pub xp: Experience,
    /// Available credits (the Zone's currency).
    pub credits: Credits,
    /// Relations with each faction, keyed by faction ID.
    pub standings: Vec<(Id<Faction>, Relation)>,
    /// Currently employed NPCs and their roles.
    pub squad: Vec<SquadMember>,
    /// Whether the Garrison bribe has been paid this period.
    pub garrison_bribe_paid: bool,
}

impl PlayerState {
    /// Create a new player state with neutral standings for all given factions.
    pub fn new(faction_ids: &[Id<Faction>]) -> Self {
        let standings = faction_ids
            .iter()
            .map(|f| (f.clone(), Relation::NEUTRAL))
            .collect();

        Self {
            xp: Experience::ZERO,
            credits: Credits::new(5000),
            standings,
            squad: Vec::new(),
            garrison_bribe_paid: false,
        }
    }

    /// Current rank, derived from XP.
    pub fn rank(&self) -> PlayerRank {
        PlayerRank::from_xp(self.xp)
    }

    /// Add experience points.
    pub fn add_xp(&mut self, amount: u32) {
        self.xp.add(amount);
    }

    /// Get the player's standing with a faction.
    pub fn standing(&self, faction: &Id<Faction>) -> Relation {
        self.standings
            .iter()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Get a mutable reference to the player's standing with a faction.
    pub fn standing_mut(&mut self, faction: &Id<Faction>) -> Option<&mut Relation> {
        self.standings
            .iter_mut()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| s)
    }

    /// Number of currently employed NPCs.
    pub fn squad_count(&self) -> u8 {
        self.squad.len() as u8
    }

    /// Whether the player can hire another squad member.
    pub fn can_hire(&self) -> bool {
        self.squad_count() < self.rank().max_squads()
    }
}
