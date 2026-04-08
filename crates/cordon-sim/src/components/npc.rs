//! Per-NPC ECS components.
//!
//! Most per-NPC data types now derive `Component` directly in
//! cordon-core (`NpcName`, `Loadout`, `Experience`, `Credits`,
//! `Personality`), so they're attached to entities without a
//! wrapper. This module only holds the cordon-sim-specific
//! components that don't have a cordon-core analog: the NPC
//! marker, baseline pool caps, the trust/loyalty scalars, the
//! perks lists, employment status, and the `NpcBundle` glue.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NpcName;
use cordon_core::entity::npc::{Npc, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::item::Loadout;
use cordon_core::primitive::{Credits, Experience, Health, Hunger, Id, Pool, Stamina, Uid};

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

/// How much this NPC trusts the player (-1.0 to 1.0). Component
/// because the underlying data is a raw `f32` and raw floats
/// can't be queried directly.
#[derive(Component, Debug, Clone, Copy)]
pub struct Trust(pub f32);

/// Squad-level loyalty (-1.0 to 1.0). Same reasoning as `Trust`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Loyalty(pub f32);

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
/// to a fresh entity.
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
    pub trust: Trust,
    pub loyalty: Loyalty,
    pub personality: Personality,
    pub perks: Perks,
    pub employment: Employment,
}

impl NpcBundle {
    /// Construct an [`NpcBundle`] from a freshly-rolled [`Npc`].
    pub fn from_npc(npc: Npc) -> Self {
        let hp_max = npc.health.max();
        Self {
            marker: NpcMarker,
            id: npc.id,
            name: npc.name,
            faction: FactionId(npc.faction),
            xp: npc.xp,
            hp: npc.health,
            stamina: StaminaPool::full(),
            hunger: HungerPool::full(),
            base_maxes: BaseMaxes {
                hp: hp_max,
                stamina: 100,
                hunger: 100,
            },
            loadout: npc.loadout,
            wealth: npc.wealth,
            trust: Trust(npc.trust),
            loyalty: Loyalty(npc.loyalty),
            personality: npc.personality,
            perks: Perks {
                all: npc.perks,
                revealed: npc.revealed_perks,
            },
            employment: Employment {
                role: npc.role,
                daily_pay: npc.daily_pay,
            },
        }
    }
}
