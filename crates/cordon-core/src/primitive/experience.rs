//! Experience points for players and NPCs.

use serde::{Deserialize, Serialize};

use super::Rank;

/// Accumulated experience points.
///
/// Used for both the player (determines [`PlayerRank`](crate::entity::player::PlayerRank))
/// and NPCs (determines [`Rank`]). XP only goes up — it never decays.
///
/// NPCs gain XP from successful missions (runners) and survived raids
/// (guards). The player gains XP from trades, completed quests, surviving
/// events, and faction milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Experience(u32);

impl Experience {
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

    /// Derive NPC rank from this experience value.
    pub fn npc_rank(self) -> Rank {
        Rank::from_xp(self)
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
