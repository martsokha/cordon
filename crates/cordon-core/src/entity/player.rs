//! Player state: rank, experience, credits, faction standings, and
//! the bunker's upgrade + storage state.
//!
//! Hired squads live in the sim layer (`PlayerSquadRoster`), not
//! here — `PlayerState` is the persistent player profile, not a
//! god-object.

use serde::{Deserialize, Serialize};

use super::bunker::Upgrade;
use super::faction::Faction;
use crate::item::{Item, Stash, StashScope};
use crate::primitive::{Credits, Day, Experience, Id, Relation};

/// A categorised daily expense line item. Multiple line items
/// compose a [`DailyExpenseReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseLine {
    pub kind: ExpenseKind,
    pub amount: Credits,
}

/// What a daily expense pays for. New cost categories are added
/// here; the payroll system in cordon-sim produces the lines,
/// and the UI in cordon-bevy reads them for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpenseKind {
    /// Per-member pay for hired squads, summed from `Rank::pay`.
    SquadUpkeep,
    /// Protection money to the Garrison faction.
    GarrisonBribe,
    /// Interest on outstanding [`PlayerState::debt`].
    SyndicateInterest,
}

/// Snapshot of one day's expenses. Produced by the payroll system
/// on each day rollover and stored in a resource so the UI can
/// display "last day's costs" at any time.
#[derive(Debug, Clone)]
pub struct DailyExpenseReport {
    pub day: Day,
    pub lines: Vec<ExpenseLine>,
    pub total: Credits,
    /// Portion of the total that couldn't be covered by available
    /// credits and was added to [`PlayerState::debt`].
    pub shortfall: Credits,
}

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

/// The player's complete state: identity, economy, faction relations,
/// and the bunker (storage + installed upgrades).
///
/// Hired-squad ownership is *not* stored here — it lives in the
/// sim-layer `PlayerSquadRoster` resource, keyed by `Uid<Squad>`.
///
/// `BaseState` was previously a separate field on `World`; it's now
/// inlined here so "the player" is a single source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Accumulated experience. Rank is derived from this.
    pub xp: Experience,
    /// Available credits (the Zone's currency).
    pub credits: Credits,
    /// Accumulated unpaid expenses from previous days. Carried
    /// forward each day-rollover; reduced when the player earns
    /// enough to cover it. Separate from `credits` so spending
    /// and owing are distinct — the player can have cash *and*
    /// debt simultaneously (e.g. earns 200 but owes 500).
    pub debt: Credits,
    /// Relations with each faction, keyed by faction ID.
    pub standings: Vec<(Id<Faction>, Relation)>,
    /// All installed upgrade IDs (both bunker and camp).
    pub upgrades: Vec<Id<Upgrade>>,
    /// Items waiting to be placed on a rack slot. Quest
    /// consequences push here; a bevy-side drain system moves
    /// them onto the first available rack slot each frame.
    /// Should be empty most of the time — not real storage.
    pub pending_items: Stash,
    /// Hidden storage (survives raids, invisible during inspections).
    pub hidden_storage: Stash,
}

impl PlayerState {
    /// Create a new player state with neutral standings for all given factions
    /// and an empty bunker.
    pub fn new(faction_ids: &[Id<Faction>]) -> Self {
        let standings = faction_ids
            .iter()
            .map(|f| (f.clone(), Relation::NEUTRAL))
            .collect();

        Self {
            xp: Experience::ZERO,
            credits: Credits::new(5000),
            debt: Credits::new(0),
            standings,
            upgrades: Vec::new(),
            pending_items: Stash::new(),
            hidden_storage: Stash::new(),
        }
    }

    /// Current rank, derived from XP.
    pub fn rank(&self) -> PlayerRank {
        PlayerRank::from_xp(self.xp)
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

    /// Check if an upgrade is installed (bunker or camp).
    pub fn has_upgrade(&self, upgrade_id: &Id<Upgrade>) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Whether the player holds at least `count` of the given item
    /// def within the scope.
    pub fn has_item(&self, item: &Id<Item>, count: u32, scope: StashScope) -> bool {
        self.item_count(item, scope) >= count
    }

    /// Total count of a given item definition across the requested scope.
    pub fn item_count(&self, item: &Id<Item>, scope: StashScope) -> u32 {
        let sum = |stash: &Stash| -> u32 {
            stash
                .items()
                .iter()
                .filter(|i| &i.def_id == item)
                .map(|i| i.count)
                .sum()
        };
        match scope {
            StashScope::Main => sum(&self.pending_items),
            StashScope::Hidden => sum(&self.hidden_storage),
            StashScope::Any => sum(&self.pending_items) + sum(&self.hidden_storage),
        }
    }
}
