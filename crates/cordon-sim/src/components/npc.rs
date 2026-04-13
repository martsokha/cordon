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
use cordon_core::entity::npc::{Npc, NpcTemplate, Personality, Role};
use cordon_core::entity::perk::Perk;
use cordon_core::item::{Loadout, TimedEffect};
use cordon_core::primitive::{
    Corruption, Credits, Experience, GameTime, Health, Id, Loyalty, Pool, Stamina, Trust, Uid,
};

/// Health pool component (current + max HP).
pub type HealthPool = Pool<Health>;

/// Stamina pool component.
pub type StaminaPool = Pool<Stamina>;

/// Corruption pool component. Accumulates from zero. Unlike
/// health/stamina which drain from full, this fills up from
/// corrupted areas, tainted food, and carried artifacts that
/// bleed Zone-stuff into their carrier, and drains back down
/// when the carrier uses an antidote or equips a scrubber
/// relic.
pub type CorruptionPool = Pool<Corruption>;

/// Per-entity list of currently-active [`TimedEffect`]s.
///
/// Populated by the effect dispatcher and drained as each
/// entry's duration expires. Instant effects never land here —
/// they apply synchronously at insertion time inside the
/// dispatcher. See [`crate::effects`] for the systems.
///
/// An active effect has no memory of its source (consumable,
/// relic trigger, throwable). Once it lands it runs out its
/// lifetime regardless of what equipment changes the carrier
/// makes — a heal-over-time from a relic still finishes even
/// if the relic is unequipped mid-tick. Adding a source field
/// later would let us cancel mid-flight on equipment change,
/// but that's not the current behaviour.
#[derive(Component, Debug, Default)]
pub struct ActiveEffects {
    pub effects: Vec<ActiveEffect>,
}

/// One active timed effect entry.
#[derive(Debug, Clone)]
pub struct ActiveEffect {
    /// The effect payload. Its `duration` is the *total*
    /// lifetime; the tick compares against elapsed minutes to
    /// decide when to stop.
    pub effect: TimedEffect,
    /// Minute the effect was added.
    pub started_at: GameTime,
    /// Minute of the last applied tick.
    pub last_tick_at: GameTime,
}

/// Baseline pool caps before any equipment bonuses.
///
/// `HealthPool` and `StaminaPool` hold the *effective* current / max;
/// this component stores the underlying base so the
/// `sync_pool_maxes` system can recompute the effective max each
/// time the loadout changes (equip +10 max HP relic → effective
/// 110, drop it → effective 100). Using a snapshot of the base
/// decouples the bookkeeping from the equipment change order.
#[derive(Component, Debug, Clone, Copy)]
pub struct BaseMaxes {
    pub hp: u32,
    pub stamina: u32,
}

impl Default for BaseMaxes {
    fn default() -> Self {
        Self {
            hp: 100,
            stamina: 100,
        }
    }
}

/// Marker that this entity is an NPC. Use as a query filter.
#[derive(Component, Debug, Clone, Copy)]
pub struct NpcMarker;

/// Links an NPC entity back to the [`NpcTemplateDef`] it was
/// spawned from. Present only on template-spawned NPCs, not on
/// generic archetype-rolled ones.
#[derive(Component, Debug, Clone)]
pub struct TemplateId(pub Id<NpcTemplate>);

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
    pub hp: HealthPool,
    pub stamina: StaminaPool,
    pub corruption: CorruptionPool,
    pub active_effects: ActiveEffects,
    pub base_maxes: BaseMaxes,
    pub loadout: Loadout,
    pub wealth: Credits,
    pub attributes: NpcAttributes,
    pub perks: Perks,
    pub employment: Employment,
}
