//! Player state: rank, credits, faction standings, and squads.

use serde::{Deserialize, Serialize};

use crate::entity::faction::Standing;
use crate::entity::npc::Role;
use crate::object::id::{Id, Uid};

/// Player rank definition loaded from config.
///
/// Defines what rank tier the player can achieve and how many
/// squads (hired NPCs) they can maintain at that rank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRankDef {
    /// Rank tier number (1–5).
    pub tier: u8,
    /// Display title (e.g., "Nobody", "Known", "Legend").
    pub title: String,
    /// Maximum number of squads (runners + guards) at this rank.
    pub max_squads: u8,
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
    /// Current rank tier (1–5). Maps to a [`PlayerRankDef`] from config.
    pub rank_tier: u8,
    /// Available credits (the Zone's currency).
    pub credits: u32,
    /// Standings with each faction, keyed by faction ID.
    pub standings: Vec<(Id, Standing)>,
    /// Currently employed NPCs and their roles.
    pub squad: Vec<SquadMember>,
    /// Whether the Garrison bribe has been paid this period.
    pub garrison_bribe_paid: bool,
}

impl PlayerState {
    /// Create a new player state with neutral standings for all given factions.
    pub fn new(faction_ids: &[Id]) -> Self {
        let standings = faction_ids
            .iter()
            .map(|f| (f.clone(), Standing::neutral()))
            .collect();

        Self {
            rank_tier: 1,
            credits: 5000,
            standings,
            squad: Vec::new(),
            garrison_bribe_paid: false,
        }
    }

    /// Get the player's standing with a faction.
    pub fn standing(&self, faction: &Id) -> Standing {
        self.standings
            .iter()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Get a mutable reference to the player's standing with a faction.
    pub fn standing_mut(&mut self, faction: &Id) -> Option<&mut Standing> {
        self.standings
            .iter_mut()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| s)
    }

    /// Number of currently employed NPCs.
    pub fn squad_count(&self) -> u8 {
        self.squad.len() as u8
    }
}
