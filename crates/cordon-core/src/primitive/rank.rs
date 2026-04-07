//! NPC rank tier (Novice → Legend), derived from experience.

use serde::{Deserialize, Serialize};

use super::Experience;

/// NPC rank tier. All factions use this 5-step scale internally — the
/// localized title (`Grunt` vs `Pilgrim` vs `Recruit`) is resolved
/// through the faction's [`RankScheme`](crate::entity::faction::RankScheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rank {
    Novice,
    Experienced,
    Veteran,
    Master,
    Legend,
}

impl Rank {
    /// XP threshold required to reach this rank.
    pub fn xp_threshold(self) -> u32 {
        match self {
            Rank::Novice => 0,
            Rank::Experienced => 1000,
            Rank::Veteran => 2500,
            Rank::Master => 5000,
            Rank::Legend => 10000,
        }
    }

    /// Numeric tier (1–5), useful for arithmetic like rank-based scaling.
    pub fn tier(self) -> u8 {
        match self {
            Rank::Novice => 1,
            Rank::Experienced => 2,
            Rank::Veteran => 3,
            Rank::Master => 4,
            Rank::Legend => 5,
        }
    }

    /// Lowercase identifier suitable for use as a localization key
    /// suffix or JSON key (e.g. `"novice"`, `"legend"`).
    pub fn key(self) -> &'static str {
        match self {
            Rank::Novice => "novice",
            Rank::Experienced => "experienced",
            Rank::Veteran => "veteran",
            Rank::Master => "master",
            Rank::Legend => "legend",
        }
    }

    /// Determine the rank from accumulated experience.
    pub fn from_xp(xp: Experience) -> Self {
        let v = xp.value();
        if v >= Rank::Legend.xp_threshold() {
            Rank::Legend
        } else if v >= Rank::Master.xp_threshold() {
            Rank::Master
        } else if v >= Rank::Veteran.xp_threshold() {
            Rank::Veteran
        } else if v >= Rank::Experienced.xp_threshold() {
            Rank::Experienced
        } else {
            Rank::Novice
        }
    }

    /// All ranks in ascending order.
    pub fn all() -> [Rank; 5] {
        [
            Rank::Novice,
            Rank::Experienced,
            Rank::Veteran,
            Rank::Master,
            Rank::Legend,
        ]
    }
}
