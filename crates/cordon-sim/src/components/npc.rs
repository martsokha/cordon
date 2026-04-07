//! Per-NPC ECS components.
//!
//! Every NPC is a Bevy entity with the components below. The
//! `cordon-core` `Npc` struct is the spawn-time / save-game shape,
//! consumed by [`NpcBundle::from_npc`].

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NpcName;
use cordon_core::entity::npc::{Npc, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::item::Loadout;
use cordon_core::primitive::{Credits, Experience, Health, Hunger, Id, Pool, Rank, Stamina, Uid};

/// Health pool component (current + max HP).
pub type Hp = Pool<Health>;

/// Stamina pool component.
pub type StaminaPool = Pool<Stamina>;

/// Hunger pool component. At max = fully satiated, at 0 = starving.
pub type HungerPool = Pool<Hunger>;

/// Baseline pool caps before any equipment bonuses.
///
/// `Hp`, `StaminaPool`, and `HungerPool` hold the *effective* current
/// / max; this component stores the underlying base so the
/// `sync_pool_maxes` system can recompute the effective max each
/// time the loadout changes (equip +10 max HP relic → effective 110,
/// drop it → effective 100). Using a snapshot of the base decouples
/// the bookkeeping from the equipment change order.
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

/// Localized name. Wrapper avoids shadowing `bevy::prelude::Name`.
#[derive(Component, Debug, Clone)]
pub struct NpcNameComp(pub NpcName);

#[derive(Component, Debug, Clone)]
pub struct FactionId(pub Id<Faction>);

#[derive(Component, Debug, Clone, Copy)]
pub struct Xp(pub Experience);

impl Xp {
    pub fn rank(&self) -> Rank {
        self.0.npc_rank()
    }
}

#[derive(Component, Debug, Clone)]
pub struct LoadoutComp(pub Loadout);

#[derive(Component, Debug, Clone, Copy)]
pub struct Wealth(pub Credits);

#[derive(Component, Debug, Clone, Copy)]
pub struct Trust(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct Loyalty(pub f32);

#[derive(Component, Debug, Clone, Copy)]
pub struct PersonalityComp(pub Personality);

#[derive(Component, Debug, Clone)]
pub struct Perks {
    pub all: Vec<Id<Perk>>,
    pub revealed: Vec<Id<Perk>>,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Employment {
    pub role: Option<Role>,
    pub daily_pay: Credits,
}

/// Bundle of every per-NPC component the spawn system attaches to a
/// fresh entity.
#[derive(Bundle)]
pub struct NpcBundle {
    pub marker: NpcMarker,
    pub id: Uid<Npc>,
    pub name: NpcNameComp,
    pub faction: FactionId,
    pub xp: Xp,
    pub hp: Hp,
    pub stamina: StaminaPool,
    pub hunger: HungerPool,
    pub base_maxes: BaseMaxes,
    pub loadout: LoadoutComp,
    pub wealth: Wealth,
    pub trust: Trust,
    pub loyalty: Loyalty,
    pub personality: PersonalityComp,
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
            name: NpcNameComp(npc.name),
            faction: FactionId(npc.faction),
            xp: Xp(npc.xp),
            hp: npc.health,
            stamina: StaminaPool::full(),
            hunger: HungerPool::full(),
            base_maxes: BaseMaxes {
                hp: hp_max,
                stamina: 100,
                hunger: 100,
            },
            loadout: LoadoutComp(npc.loadout),
            wealth: Wealth(npc.wealth),
            trust: Trust(npc.trust),
            loyalty: Loyalty(npc.loyalty),
            personality: PersonalityComp(npc.personality),
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
