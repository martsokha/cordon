//! ECS components for NPCs and squads.
//!
//! These replace the old `World.npcs: HashMap<Uid<Npc>, Npc>` model:
//! every NPC is a Bevy entity with the components below, and every
//! squad is an entity with the squad components. The hashmaps in
//! `World` are gone.
//!
//! `Uid<Npc>` and `Uid<Squad>` still exist as stable identifiers for
//! save-game and quest references, but runtime systems use `Entity`
//! to look things up because it's an O(1) array index in Bevy.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NpcName;
use cordon_core::entity::npc::{Npc, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::item::Loadout;
use cordon_core::primitive::{Credits, Experience, Health, Id, Rank, Uid};

// ====================================================================
// Per-NPC components
// ====================================================================

/// Marker that this entity is an NPC. Use as a query filter.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcMarker;

/// Stable runtime identifier. Persists across this game session and
/// is the key used in save files; for *runtime* lookups, prefer the
/// entity itself.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcId(pub Uid<Npc>);

#[derive(Component, Debug, Clone)]
pub struct Name(pub NpcName);

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

/// Bundle that bundles all per-NPC components together. Used by the
/// spawn system: it constructs an `Npc` value (still defined in
/// `cordon-core` for save-game/serde) and then unpacks it into this
/// bundle when spawning the entity.
#[derive(Bundle)]
pub struct NpcBundle {
    pub marker: NpcMarker,
    pub id: NpcId,
    pub name: Name,
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
            name: Name(npc.name),
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

// ====================================================================
// Per-squad components
// ====================================================================

/// Marker that this entity is a squad.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMarker;

/// Stable runtime identifier for the squad.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadId(pub Uid<Squad>);

#[derive(Component, Debug, Clone)]
pub struct SquadFaction(pub Id<Faction>);

#[derive(Component, Debug, Clone)]
pub struct SquadGoal(pub Goal);

#[derive(Component, Debug, Clone, Copy)]
pub struct SquadFormation(pub Formation);

/// Last known facing direction for formation rotation. Default is +Y.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadFacing(pub Vec2);

impl Default for SquadFacing {
    fn default() -> Self {
        Self(Vec2::Y)
    }
}

/// Patrol/scavenge waypoints inside the goal area + the index of the
/// next one to visit. Empty for non-patrol goals.
#[derive(Component, Debug, Clone, Default)]
pub struct SquadWaypoints {
    pub points: Vec<Vec2>,
    pub next: u8,
}

/// Initial spawn position for the squad, used by the visual layer to
/// place freshly-spawned members at the right map coordinate before
/// the formation system takes over.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadHomePosition(pub Vec2);

/// The current leader's entity. Promoted to highest-rank survivor
/// when the previous leader dies.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadLeader(pub Entity);

/// All member entities (alive). Pruned by `cleanup_dead_squads`.
#[derive(Component, Debug, Clone)]
pub struct SquadMembers(pub Vec<Entity>);

/// Short-term squad activity (Hold / Move / Engage). The squad
/// systems read and write this each tick.
#[derive(Component, Debug, Clone)]
pub enum SquadActivity {
    Hold { duration_secs: f32 },
    Move { target: Vec2 },
    Engage { hostiles: Entity },
}

impl Default for SquadActivity {
    fn default() -> Self {
        Self::Hold { duration_secs: 1.0 }
    }
}

#[derive(Bundle)]
pub struct SquadBundle {
    pub marker: SquadMarker,
    pub id: SquadId,
    pub faction: SquadFaction,
    pub goal: SquadGoal,
    pub formation: SquadFormation,
    pub facing: SquadFacing,
    pub waypoints: SquadWaypoints,
    pub home: SquadHomePosition,
    pub leader: SquadLeader,
    pub members: SquadMembers,
    pub activity: SquadActivity,
}

impl SquadBundle {
    pub fn from_squad(
        squad: Squad,
        leader: Entity,
        members: Vec<Entity>,
        home: Vec2,
    ) -> Self {
        Self {
            marker: SquadMarker,
            id: SquadId(squad.id),
            faction: SquadFaction(squad.faction),
            goal: SquadGoal(squad.goal),
            formation: SquadFormation(squad.formation),
            facing: SquadFacing(Vec2::new(squad.facing[0], squad.facing[1])),
            waypoints: SquadWaypoints {
                points: squad
                    .waypoints
                    .into_iter()
                    .map(|p| Vec2::new(p[0], p[1]))
                    .collect(),
                next: squad.next_waypoint,
            },
            home: SquadHomePosition(home),
            leader: SquadLeader(leader),
            members: SquadMembers(members),
            activity: SquadActivity::default(),
        }
    }
}

/// Back-pointer from an NPC entity to its squad entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct SquadMembership {
    pub squad: Entity,
    /// Formation slot index (0 = leader, 1..=4 = followers).
    pub slot: u8,
}
