//! Per-NPC ECS components.
//!
//! Most per-NPC data types now derive `Component` directly in
//! cordon-core (`NpcName`, `Loadout`, `Experience`, `Credits`,
//! `Personality`, `Trust`, `Loyalty`), so they're attached to
//! entities without a wrapper. This module only holds the
//! cordon-sim-specific components that don't have a cordon-core
//! analog: the NPC marker, baseline pool caps, the perks lists,
//! employment status, and the `NpcBundle` glue.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NpcName;
use cordon_core::entity::npc::{Npc, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::item::Loadout;
use cordon_core::primitive::{
    Credits, Experience, Health, Hunger, Id, Loyalty, Pool, Stamina, Trust, Uid,
};

/// Health pool component (current + max HP).
pub type Hp = Pool<Health>;

/// Stamina pool component.
pub type StaminaPool = Pool<Stamina>;

/// Hunger pool component. At max = fully satiated, at 0 = starving.
pub type HungerPool = Pool<Hunger>;

/// Baseline pool caps before any equipment bonuses.
///
/// `Hp`, `StaminaPool`, and `HungerPool` hold the *effective*
/// current / max; this component stores the underlying base so
/// the `sync_pool_maxes` system can recompute the effective max
/// each time the loadout changes (equip +10 max HP relic →
/// effective 110, drop it → effective 100). Using a snapshot of
/// the base decouples the bookkeeping from the equipment change
/// order.
#[derive(Component, Debug, Clone, Copy)]
pub struct BaseMaxes {
    pub hp: u32,
    pub stamina: u32,
    pub hunger: u32,
}

impl Default for BaseMaxes {
    fn default() -> Self {
        Self {
            hp: 100,
            stamina: 100,
            hunger: 100,
        }
    }
}

/// Marker that this entity is an NPC. Use as a query filter.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcMarker;

/// Faction membership. Distinct from `Id<Faction>` in other
/// contexts (e.g. fields inside data structs) because this
/// wrapper type is what Bevy queries actually filter on —
/// `With<FactionId>` only matches entities, not raw IDs.
#[derive(Component, Debug, Clone)]
pub struct FactionId(pub Id<Faction>);

/// Hidden NPC attributes affecting negotiation and squad
/// behaviour. Bundled into one component because a query for
/// "how does this NPC feel" always wants all of these at once —
/// splitting them into three separate components would force
/// three query touches for every decision that depends on NPC
/// mood.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NpcAttributes {
    pub trust: Trust,
    pub loyalty: Loyalty,
    pub personality: Personality,
}

/// Perk lists. Cordon-core's `Npc` stores `perks` and
/// `revealed_perks` as two separate `Vec`s; we bundle them into
/// one component here because a query for "NPC's perks" always
/// wants both at once.
#[derive(Component, Debug, Clone)]
pub struct Perks {
    pub all: Vec<Id<Perk>>,
    pub revealed: Vec<Id<Perk>>,
}

/// Employment status. Bundles the two employment fields from
/// cordon-core's `Npc` into one component so "is this NPC
/// hired?" is a single query touch.
#[derive(Component, Debug, Clone, Copy)]
pub struct Employment {
    pub role: Option<Role>,
    pub daily_pay: Credits,
}

/// Bundle of every per-NPC component the spawn system attaches
/// to a fresh entity. Built directly by the generator — there's
/// no intermediate `Npc` data struct any more.
#[derive(Bundle)]
pub struct NpcBundle {
    pub marker: NpcMarker,
    pub id: Uid<Npc>,
    pub name: NpcName,
    pub faction: FactionId,
    pub xp: Experience,
    pub hp: Hp,
    pub stamina: StaminaPool,
    pub hunger: HungerPool,
    pub base_maxes: BaseMaxes,
    pub loadout: Loadout,
    pub wealth: Credits,
    pub attributes: NpcAttributes,
    pub perks: Perks,
    pub employment: Employment,
}
