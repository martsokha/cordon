//! Player state: rank, credits, faction standings, and squads.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Standing;
use crate::entity::npc::Role;
use crate::primitive::id::{Id, Faction};
use crate::primitive::uid::Uid;

/// Player rank tier. Determines squad capacity and unlocks.
///
/// Ranking up is earned through gameplay — trade volume, faction
/// standing, completed missions, and surviving crises all contribute.
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
            PlayerRank::Nobody => 2,
            PlayerRank::Known => 3,
            PlayerRank::Established => 4,
            PlayerRank::Connected => 5,
            PlayerRank::Legend => 6,
        }
    }

    /// The numeric tier (1–5).
    pub fn tier(self) -> u8 {
        self as u8
    }
}

/// A hired NPC assigned to a role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadMember {
    /// Runtime UID of the hired NPC.
    pub npc_id: Uid,
    /// Whether this NPC is a runner or guard.
    pub role: Role,
}

/// The player's current state.
///
/// Tracks rank, credits, faction standings, and the squad roster.
/// Created at game start and mutated throughout gameplay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Current rank. Determines max squad size.
    pub rank: PlayerRank,
    /// Available credits (the Zone's currency).
    pub credits: u32,
    /// Standings with each faction, keyed by faction ID.
    pub standings: Vec<(Id<Faction>, Standing)>,
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
            .map(|f| (f.clone(), Standing::neutral()))
            .collect();

        Self {
            rank: PlayerRank::Nobody,
            credits: 5000,
            standings,
            squad: Vec::new(),
            garrison_bribe_paid: false,
        }
    }

    /// Get the player's standing with a faction.
    pub fn standing(&self, faction: &Id<Faction>) -> Standing {
        self.standings
            .iter()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Get a mutable reference to the player's standing with a faction.
    pub fn standing_mut(&mut self, faction: &Id<Faction>) -> Option<&mut Standing> {
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
        self.squad_count() < self.rank.max_squads()
    }
}
