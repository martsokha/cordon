//! Per-NPC ECS components.
//!
//! These replace the old `World.npcs: HashMap<Uid<Npc>, Npc>` model:
//! every NPC is a Bevy entity with the components below. The
//! `cordon-core` `Npc` struct still exists as the spawn-time / save-game
//! shape, consumed by [`NpcBundle::from_npc`].

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NpcName;
use cordon_core::entity::npc::{Npc, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::item::Loadout;
use cordon_core::primitive::{Credits, Experience, Health, Id, Rank, Uid};

/// Marker that this entity is an NPC. Use as a query filter.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcMarker;

/// Stable runtime identifier. Persists across this game session and
/// is the key used in save files; for *runtime* lookups, prefer the
/// entity itself.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcId(pub Uid<Npc>);

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

/// Current and max HP, both as plain integers.
#[derive(Component, Debug, Clone, Copy)]
pub struct Hp {
    pub current: Health,
    pub max: u32,
}

impl Hp {
    pub fn new(current: Health, max: u32) -> Self {
        Self { current, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current.is_alive()
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
    pub id: NpcId,
    pub name: NpcNameComp,
    pub faction: FactionId,
    pub xp: Xp,
    pub hp: Hp,
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
        let max_hp = npc.max_hp;
        Self {
            marker: NpcMarker,
            id: NpcId(npc.id),
            name: NpcNameComp(npc.name),
            faction: FactionId(npc.faction),
            xp: Xp(npc.xp),
            hp: Hp {
                current: npc.health,
                max: max_hp,
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
