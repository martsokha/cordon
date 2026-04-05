use serde::{Deserialize, Serialize};

use crate::entity::faction::{FactionId, Standing};
use crate::entity::npc::{NpcId, Role};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlayerRank {
    Nobody = 1,
    Known = 2,
    Established = 3,
    Connected = 4,
    Legend = 5,
}

impl PlayerRank {
    pub fn max_squads(self) -> u8 {
        match self {
            PlayerRank::Nobody => 2,
            PlayerRank::Known => 3,
            PlayerRank::Established => 4,
            PlayerRank::Connected => 5,
            PlayerRank::Legend => 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadMember {
    pub npc_id: NpcId,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub rank: PlayerRank,
    pub credits: u32,
    pub standings: Vec<(FactionId, Standing)>,
    pub squad: Vec<SquadMember>,
    pub garrison_bribe_paid: bool,
}

impl PlayerState {
    pub fn new() -> Self {
        let standings = FactionId::ALL
            .iter()
            .map(|&f| (f, Standing::neutral()))
            .collect();

        Self {
            rank: PlayerRank::Nobody,
            credits: 5000,
            standings,
            squad: Vec::new(),
            garrison_bribe_paid: false,
        }
    }

    pub fn standing(&self, faction: FactionId) -> Standing {
        self.standings
            .iter()
            .find(|(f, _)| *f == faction)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    pub fn standing_mut(&mut self, faction: FactionId) -> &mut Standing {
        let pos = self
            .standings
            .iter()
            .position(|(f, _)| *f == faction)
            .expect("faction not in standings");
        &mut self.standings[pos].1
    }

    pub fn squad_count(&self) -> u8 {
        self.squad.len() as u8
    }

    pub fn can_hire(&self) -> bool {
        self.squad_count() < self.rank.max_squads()
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}
