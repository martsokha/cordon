//! Experience points for players and NPCs.

use serde::{Deserialize, Serialize};

/// Accumulated experience points.
///
/// Used for both the player (determines [`PlayerRank`](crate::entity::player::PlayerRank))
/// and NPCs (determines rank tier). XP only goes up — it never decays.
///
/// NPCs gain XP from successful missions (runners) and survived raids
/// (guards). The player gains XP from trades, completed quests, surviving
/// events, and faction milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Experience(u32);

impl Experience {
    /// Define NPC rank thresholds.
    const NPC_RANK_THRESHOLDS: [u32; 5] = [0, 1000, 2500, 5000, 10000];
    /// Zero experience.
    pub const ZERO: Self = Self(0);

    /// Create from a raw value.
    pub fn new(xp: u32) -> Self {
        Self(xp)
    }

    /// Get the raw XP value.
    pub fn value(self) -> u32 {
        self.0
    }

    /// Add experience. Saturates at `u32::MAX`.
    pub fn add(&mut self, amount: u32) {
        self.0 = self.0.saturating_add(amount);
    }

    /// Whether this experience meets or exceeds a threshold.
    pub fn meets(self, threshold: u32) -> bool {
        self.0 >= threshold
    }

    /// Derive NPC rank tier (1–5) from this experience value.
    pub fn npc_rank(self) -> u8 {
        if self.0 >= Self::NPC_RANK_THRESHOLDS[4] {
            5
        } else if self.0 >= Self::NPC_RANK_THRESHOLDS[3] {
            4
        } else if self.0 >= Self::NPC_RANK_THRESHOLDS[2] {
            3
        } else if self.0 >= Self::NPC_RANK_THRESHOLDS[1] {
            2
        } else {
            1
        }
    }
}

impl From<u32> for Experience {
    fn from(xp: u32) -> Self {
        Self(xp)
    }
}

impl std::fmt::Display for Experience {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} XP", self.0)
    }
}
