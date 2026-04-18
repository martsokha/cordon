//! Player-owned run state.
//!
//! Everything the UI, quest consequences, and shop flow read or
//! write about the current character: numeric identity, faction
//! standings, installed upgrades, stash, intel, hired squads.
//! [`assemble_player_state`] collects the subset that belongs in
//! a save snapshot.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::bunker::{Upgrade, UpgradeDef, UpgradeEffect};
use cordon_core::entity::faction::Faction;
use cordon_core::entity::player::{PlayerRank, PlayerState};
use cordon_core::entity::squad::Squad;
use cordon_core::item::{Item, ItemInstance, Stash, StashScope};
use cordon_core::primitive::{Credits, Day, Experience, Id, Relation, Uid};
use cordon_core::world::narrative::{Intel, IntelDef};

/// Per-hire bookkeeping for one squad on the player's roster.
///
/// Empty for now — the field exists so future per-hire metadata
/// (date hired, custom callsign, assignment) can be added without
/// changing [`PlayerSquadRoster`]'s shape.
///
/// Daily pay is intentionally **not** stored here — it's a pure
/// function of the squad's current member ranks (see
/// [`Rank::pay`](cordon_core::primitive::Rank::pay)) so member
/// deaths immediately reduce the bill with no recompute system.
#[derive(Debug, Clone, Default)]
pub struct PlayerSquadEntry {}

/// All squads the player has hired. Squads are the only unit of
/// player ownership — there is no individual NPC hiring.
///
/// Keyed by stable [`Uid<Squad>`] so the roster survives respawns
/// and is save-ready. ECS systems that need entity access find
/// the live entity via [`SquadIdIndex`](super::clock::SquadIdIndex)
/// (or, more commonly, through the derived
/// [`Owned`](crate::behavior::squad::Owned) marker which is kept
/// in sync by [`sync_owned_marker`]).
#[derive(Resource, Default, Debug, Clone)]
pub struct PlayerSquadRoster {
    entries: HashMap<Uid<Squad>, PlayerSquadEntry>,
}

impl PlayerSquadRoster {
    /// Add a squad to the roster. No-op if already hired.
    pub fn hire(&mut self, squad: Uid<Squad>) {
        self.entries.entry(squad).or_default();
    }

    /// Remove a squad from the roster. No-op if not hired.
    pub fn dismiss(&mut self, squad: &Uid<Squad>) {
        self.entries.remove(squad);
    }

    pub fn is_hired(&self, squad: &Uid<Squad>) -> bool {
        self.entries.contains_key(squad)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Uid<Squad>, &PlayerSquadEntry)> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// XP, credits, debt — the player's numeric identity.
#[derive(Resource, Debug, Clone)]
pub struct PlayerIdentity {
    pub xp: Experience,
    pub credits: Credits,
    pub debt: Credits,
}

impl PlayerIdentity {
    /// Current rank, derived from XP.
    pub fn rank(&self) -> PlayerRank {
        PlayerRank::from_xp(self.xp)
    }

    /// Add experience points.
    pub fn add_xp(&mut self, amount: u32) {
        self.xp.add(amount);
    }

    /// Whether the player can afford a given cost.
    pub fn can_afford(&self, amount: Credits) -> bool {
        self.credits.can_afford(amount)
    }
}

/// Faction relations.
#[derive(Resource, Debug, Clone)]
pub struct PlayerStandings {
    pub standings: Vec<(Id<Faction>, Relation)>,
}

impl PlayerStandings {
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
}

/// Installed bunker/camp upgrades.
#[derive(Resource, Debug, Clone)]
pub struct PlayerUpgrades {
    pub upgrades: Vec<Id<Upgrade>>,
}

impl PlayerUpgrades {
    /// Check if an upgrade is installed (bunker or camp).
    pub fn has_upgrade(&self, upgrade_id: &Id<Upgrade>) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Iterate every [`UpgradeEffect`] granted by the player's
    /// currently-installed upgrades, resolved against the game
    /// data catalog.
    pub fn installed_effects<'a>(
        &'a self,
        upgrades: &'a HashMap<Id<Upgrade>, UpgradeDef>,
    ) -> impl Iterator<Item = &'a UpgradeEffect> + 'a {
        self.upgrades
            .iter()
            .filter_map(|id| upgrades.get(id))
            .flat_map(|def| def.effects.iter())
    }
}

/// Item staging queue + hidden storage.
#[derive(Resource, Debug, Clone)]
pub struct PlayerStash {
    pub pending_items: Stash,
    pub hidden_storage: Stash,
}

impl PlayerStash {
    /// Insert an item instance into the requested scope.
    pub fn add_item(&mut self, instance: ItemInstance, scope: StashScope) {
        match scope {
            StashScope::Main | StashScope::Any => self.pending_items.add(instance),
            StashScope::Hidden => self.hidden_storage.add(instance),
        }
    }

    /// Remove and return the first instance of the given item def
    /// within the scope, or `None` if nothing matches.
    pub fn remove_first(&mut self, item: &Id<Item>, scope: StashScope) -> Option<ItemInstance> {
        let take_from = |stash: &mut Stash| -> Option<ItemInstance> {
            let index = stash.items().iter().position(|i| &i.def_id == item)?;
            stash.remove(index)
        };
        match scope {
            StashScope::Main => take_from(&mut self.pending_items),
            StashScope::Hidden => take_from(&mut self.hidden_storage),
            StashScope::Any => {
                take_from(&mut self.pending_items).or_else(|| take_from(&mut self.hidden_storage))
            }
        }
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

    /// Whether the player holds at least `count` of the given item
    /// def within the scope.
    pub fn has_item(&self, item: &Id<Item>, count: u32, scope: StashScope) -> bool {
        self.item_count(item, scope) >= count
    }
}

/// Assemble a full [`PlayerState`] DTO from the four sub-resources.
/// Used for save/load serialisation.
pub fn assemble_player_state(
    id: &PlayerIdentity,
    st: &PlayerStandings,
    up: &PlayerUpgrades,
    stash: &PlayerStash,
) -> PlayerState {
    PlayerState {
        xp: id.xp,
        credits: id.credits,
        debt: id.debt,
        standings: st.standings.clone(),
        upgrades: up.upgrades.clone(),
        pending_items: stash.pending_items.clone(),
        hidden_storage: stash.hidden_storage.clone(),
    }
}

/// One piece of intel the player has discovered.
#[derive(Debug, Clone)]
pub struct KnownIntel {
    /// Which intel definition this is.
    pub id: Id<Intel>,
    /// The day the player learned this intel.
    pub day_acquired: Day,
}

/// All intel entries the player currently knows. Populated by
/// radio broadcasts, quest consequences, and dialogue.
#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerIntel {
    pub entries: Vec<KnownIntel>,
}

impl PlayerIntel {
    /// Whether the player already knows this intel entry.
    pub fn has(&self, id: &Id<Intel>) -> bool {
        self.entries.iter().any(|e| &e.id == id)
    }

    /// Grant an intel entry. No-op if already known.
    pub fn grant(&mut self, id: Id<Intel>, day: Day) {
        if !self.has(&id) {
            self.entries.push(KnownIntel {
                id,
                day_acquired: day,
            });
        }
    }

    /// Remove expired entries given the current day and the intel
    /// catalog. Entries whose definition has `expires_after: Some(d)`
    /// are pruned when the elapsed days since acquisition exceed `d`
    /// converted to whole days. Runs on day rollover, so day
    /// granularity is appropriate.
    pub fn expire(&mut self, current_day: Day, defs: &HashMap<Id<Intel>, IntelDef>) {
        self.entries.retain(|entry| {
            let Some(def) = defs.get(&entry.id) else {
                return true;
            };
            let Some(ttl) = def.expires_after else {
                return true;
            };
            let elapsed_days = current_day
                .value()
                .saturating_sub(entry.day_acquired.value());
            // Duration stores minutes; convert to whole days
            // (rounding up so a 1-hour TTL still survives at
            // least until the next day rollover).
            let ttl_days = (ttl.minutes() + 24 * 60 - 1) / (24 * 60);
            elapsed_days < ttl_days
        });
    }
}
