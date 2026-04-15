//! Player state: rank, experience, credits, faction standings, and
//! the bunker's upgrade + storage state.
//!
//! Hired squads live in the sim layer (`PlayerSquadRoster`), not
//! here — `PlayerState` is the persistent player profile, not a
//! god-object.

use serde::{Deserialize, Serialize};

use super::bunker::{Upgrade, UpgradeDef, UpgradeEffect};
use super::faction::Faction;
use crate::item::{Item, ItemInstance, Stash, StashScope};
use crate::primitive::{Credits, Experience, Id, Relation};

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
    /// Relations with each faction, keyed by faction ID.
    pub standings: Vec<(Id<Faction>, Relation)>,
    /// Whether the Garrison bribe has been paid this period.
    pub garrison_bribe_paid: bool,
    /// All installed upgrade IDs (both bunker and camp).
    pub upgrades: Vec<Id<Upgrade>>,
    /// Main bunker storage.
    pub storage: Stash,
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
            standings,
            garrison_bribe_paid: false,
            upgrades: Vec::new(),
            storage: Stash::new(),
            hidden_storage: Stash::new(),
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

    /// Check if an upgrade is installed (bunker or camp).
    pub fn has_upgrade(&self, upgrade_id: &Id<Upgrade>) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Iterate every [`UpgradeEffect`] granted by the player's
    /// currently-installed upgrades, resolved against the game
    /// data catalog.
    ///
    /// Systems that care about mechanical upgrade outcomes should
    /// query this instead of matching on specific upgrade IDs —
    /// keeps the code data-driven and lets new upgrades slot in by
    /// declaring the same effect variant.
    pub fn installed_effects<'a>(
        &'a self,
        upgrades: &'a std::collections::HashMap<Id<Upgrade>, UpgradeDef>,
    ) -> impl Iterator<Item = &'a UpgradeEffect> + 'a {
        self.upgrades
            .iter()
            .filter_map(|id| upgrades.get(id))
            .flat_map(|def| def.effects.iter())
    }

    /// Total count of a given item definition across the requested
    /// scope. For weapons and consumables this counts *instances*
    /// (one per entry in the stash); for ammo it sums the `count`
    /// field across matching instances (rounds across boxes).
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
            StashScope::Main => sum(&self.storage),
            StashScope::Hidden => sum(&self.hidden_storage),
            StashScope::Any => sum(&self.storage) + sum(&self.hidden_storage),
        }
    }

    /// Whether the player holds at least `count` of the given item
    /// def within the scope. Uses [`item_count`](Self::item_count)
    /// semantics — one instance of a 30-round ammo box counts as 30.
    pub fn has_item(&self, item: &Id<Item>, count: u32, scope: StashScope) -> bool {
        self.item_count(item, scope) >= count
    }

    /// Insert an item instance into the requested scope.
    ///
    /// - [`Main`](StashScope::Main): main bunker storage.
    /// - [`Hidden`](StashScope::Hidden): hidden cache.
    /// - [`Any`](StashScope::Any): main bunker storage (same as
    ///   `Main` for now — kept so the scope enum stays meaningful
    ///   once physical rack placement lands and main storage
    ///   regains a real capacity).
    pub fn add_item(&mut self, instance: ItemInstance, scope: StashScope) {
        match scope {
            StashScope::Main | StashScope::Any => self.storage.add(instance),
            StashScope::Hidden => self.hidden_storage.add(instance),
        }
    }

    /// Remove and return the first instance of the given item def
    /// within the scope, or `None` if nothing matches. Under
    /// [`StashScope::Any`] main is searched first.
    pub fn remove_first(&mut self, item: &Id<Item>, scope: StashScope) -> Option<ItemInstance> {
        let take_from = |stash: &mut Stash| -> Option<ItemInstance> {
            let index = stash.items().iter().position(|i| &i.def_id == item)?;
            stash.remove(index)
        };
        match scope {
            StashScope::Main => take_from(&mut self.storage),
            StashScope::Hidden => take_from(&mut self.hidden_storage),
            StashScope::Any => {
                take_from(&mut self.storage).or_else(|| take_from(&mut self.hidden_storage))
            }
        }
    }
}
