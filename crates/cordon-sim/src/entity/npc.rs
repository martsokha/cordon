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
use cordon_core::entity::npc::{Npc, NpcTemplate, Personality};
use cordon_core::entity::perk::Perk;
use cordon_core::item::{Loadout, TimedEffect};
use cordon_core::primitive::{
    Corruption, Credits, Experience, GameTime, Health, Id, Loyalty, Pool, Stamina, Trust, Uid,
};

/// Per-entity list of currently-active [`TimedEffect`]s.
///
/// Populated by the effect dispatcher and drained as each
/// entry's duration expires. Instant effects never land here —
/// they apply synchronously at insertion time inside the
/// dispatcher. See [`crate::behavior::effects`] for the systems.
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
/// `Pool<Health>` and `Pool<Stamina>` hold the *effective* current /
/// max; this component stores the underlying base so the
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

/// Marker for a template NPC that is currently traveling to the
/// bunker. Removed on arrival. Drives arrival detection.
#[derive(Component, Debug, Clone, Copy)]
pub struct TravelingToBunker;

/// Map-space coordinate where a template NPC was spawned. Used
/// for the return-travel leg — after dialogue, they travel back
/// here before despawning.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpawnOrigin(pub Vec2);

/// Marker: this template NPC is traveling home after dialogue.
/// Driven by `detect_home_arrival`.
#[derive(Component, Debug, Clone, Copy)]
pub struct TravelingHome;

/// Marker placed on NPC entities whose map dot should render
/// with a pulsing quest-critical outline. Reserved for template
/// NPCs that matter narratively.
#[derive(Component, Debug, Clone, Copy)]
pub struct QuestCritical;

/// Yarn node this template NPC should dispatch when admitted
/// as a visitor. Set at SpawnNpc time, read on arrival.
#[derive(Component, Debug, Clone)]
pub struct PendingYarnNode(pub String);

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

/// Employment status. "Is this NPC hired?" is a single query
/// touch; `daily_pay` of zero means unemployed.
#[derive(Component, Debug, Clone, Copy)]
pub struct Employment {
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
    pub hp: Pool<Health>,
    pub stamina: Pool<Stamina>,
    pub corruption: Pool<Corruption>,
    pub active_effects: ActiveEffects,
    pub base_maxes: BaseMaxes,
    pub loadout: Loadout,
    pub wealth: Credits,
    pub attributes: NpcAttributes,
    pub perks: Perks,
    pub employment: Employment,
}
