//! Conditions and consequences shared by quests and events.
//!
//! [`ObjectiveCondition`] is the vocabulary for "something the
//! player must satisfy" — quest objectives, quest trigger
//! prerequisites, and choice gates. [`Consequence`] is the
//! vocabulary for "something that happens to the world" — quest
//! outcomes, event fallout, choice effects.
//!
//! Both enums are shared so the sim has a single condition
//! evaluator and a single consequence applier, no matter whether
//! the caller is a quest stage or an active event.
//!
//! Conditions are recursive: [`ObjectiveCondition::AllOf`],
//! [`AnyOf`](ObjectiveCondition::AnyOf), and
//! [`Not`](ObjectiveCondition::Not) compose the leaf conditions
//! into arbitrary boolean expressions.

use serde::{Deserialize, Serialize};

use super::event::Event;
use super::flag::QuestFlagPredicate;
use super::intel::Intel;
use super::quest::Quest;
use crate::entity::bunker::Upgrade;
use crate::entity::faction::Faction;
use crate::entity::npc::NpcTemplate;
use crate::item::{ItemCategory, ItemQuery};
use crate::primitive::{Credits, Duration, Experience, Id, Relation, RelationDelta};
use crate::world::area::Area;

/// A boolean condition over world state.
///
/// Used for quest objectives (what must become true for a stage
/// to succeed), quest trigger prerequisites (extra gating on top
/// of the trigger kind), and quest flag lookups. Compound
/// conditions ([`AllOf`](Self::AllOf), [`AnyOf`](Self::AnyOf),
/// [`Not`](Self::Not)) make the leaf vocabulary compose without
/// needing a custom per-quest condition type.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveCondition {
    /// Player holds at least `count` of the given item def in
    /// the scoped stash(es). See [`ItemQuery`] for the field
    /// layout and defaults.
    HaveItem(ItemQuery),
    /// Player has at least this many credits.
    HaveCredits(Credits),
    /// Player's standing with the given faction is at least the
    /// given relation.
    FactionStanding {
        faction: Id<Faction>,
        min_standing: Relation,
    },
    /// The given upgrade is installed.
    HaveUpgrade(Id<Upgrade>),
    /// The player has the given intel entry.
    HaveIntel(Id<Intel>),
    /// The given event is currently active.
    EventActive(Id<Event>),
    /// The given quest is currently active.
    QuestActive(Id<Quest>),
    /// The given quest has been completed successfully.
    QuestCompleted(Id<Quest>),
    /// A flag on the given quest matches the predicate. The
    /// evaluator reads the flag from the active quest first and
    /// falls back to the most recent completed instance — so
    /// later quests can branch on how an earlier one ended.
    ///
    /// Flag values are [`QuestFlagValue`](super::QuestFlagValue)s
    /// under the hood; the predicate is the richer vocabulary
    /// (see [`QuestFlagPredicate`]) so authors can test `IsSet`,
    /// numeric comparisons, and explicit inequality without
    /// stringly-typed coercion.
    QuestFlag {
        quest: Id<Quest>,
        key: String,
        predicate: QuestFlagPredicate,
    },
    /// The named NPC template is currently alive in the world.
    /// Stub: the evaluator warns and returns `false` until the
    /// NpcTemplate → live-entity resolution story is in.
    NpcAlive(Id<NpcTemplate>),
    /// The named NPC template has died at least once. Stub.
    NpcDead(Id<NpcTemplate>),
    /// The named NPC template is currently in the named area.
    /// Stub.
    NpcAtLocation {
        npc: Id<NpcTemplate>,
        area: Id<Area>,
    },
    /// Wait for the given duration to elapse in stage time. Used
    /// for pacing stages where no world event needs to happen
    /// but the quest shouldn't advance immediately.
    ///
    /// [`Duration::INSTANT`] is equivalent to the old unit `Wait`
    /// and evaluates to true on the first tick.
    Wait { duration: Duration },
    /// At least `days` whole days have elapsed since the player
    /// last took pills. If they've never taken pills, the count
    /// starts from day 1 — so this condition can fire on a fresh
    /// save.
    DaysWithoutPills { days: u32 },
    /// Current day number is at least `day`. Used to gate content
    /// behind a minimum in-game calendar day (e.g., "this quest
    /// can't fire before day 6 no matter what").
    DayReached { day: u32 },
    /// All of the nested conditions must be true.
    AllOf(Vec<ObjectiveCondition>),
    /// At least one of the nested conditions must be true.
    AnyOf(Vec<ObjectiveCondition>),
    /// Logical negation of the nested condition.
    Not(Box<ObjectiveCondition>),
}

/// A bundle of [`Consequence`]s with an optional guard.
///
/// Used inside [`QuestStageKind::Outcome`](super::QuestStageKind::Outcome)
/// so a single terminal stage can fork its rewards on world
/// state without needing a separate `Outcome` stage per arm.
///
/// Evaluation: when `when` is `None` the `apply` list always
/// fires; otherwise the condition is checked (with the Outcome
/// stage's clock as the stage context) and the list fires only
/// if the guard evaluates true. Guards are independent — every
/// matching arm runs, in order, so bundles compose.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConditionalConsequence {
    /// Guard condition. `None` means the bundle always fires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<ObjectiveCondition>,
    /// The consequences to apply when the guard passes.
    pub apply: Vec<Consequence>,
}

/// A mutation applied to world state.
///
/// Fired by quest outcomes, quest stage transitions, choice
/// effects, and event triggers. The sim has a single applier
/// that pattern-matches on this enum — adding a new variant
/// means adding one branch there and nowhere else.
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Consequence {
    /// Shift the player's standing with a faction.
    StandingChange {
        faction: Id<Faction>,
        delta: RelationDelta,
    },
    /// Credit the player with currency.
    GiveCredits(Credits),
    /// Debit the player's currency.
    TakeCredits(Credits),
    /// Place copies of an item into the player's stash. Each
    /// copy is a fresh [`ItemInstance`](crate::item::ItemInstance);
    /// stacks do not merge. See [`ItemQuery`] for field layout.
    GiveItem(ItemQuery),
    /// Remove copies of an item from the player's stash.
    /// Removes the first N matching instances; short-circuits
    /// with a warning if the stash runs out.
    TakeItem(ItemQuery),
    /// Fire an event by its definition ID.
    ///
    /// Authors can optionally override the fields that the daily
    /// roll would normally randomize — `target_area`,
    /// `involved_factions`, and `duration_days`. An omitted
    /// override falls through to the def's own rng path, so a
    /// minimal `{ "trigger_event": { "event": "surge" } }` stays
    /// valid. Fields that are always author-authoritative on the
    /// def side (category, probability, consequences, chain
    /// events) are not overridable here.
    TriggerEvent {
        /// Which event def to spawn.
        event: Id<Event>,
        /// Override for `ActiveEvent::target_area`. `None` falls
        /// through to the def's `target_areas` rng pick.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_area: Option<Id<Area>>,
        /// Override for `ActiveEvent::involved_factions`. An
        /// empty vec falls through to the def's `involved_factions`
        /// rng pick. Non-empty values are used verbatim.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        involved_factions: Vec<Id<Faction>>,
        /// Override for `ActiveEvent::duration_days`. `None`
        /// falls through to a random roll in the def's
        /// `min_duration..=max_duration` range.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_days: Option<u8>,
    },
    /// Start a quest manually (bypassing its trigger table).
    StartQuest(Id<Quest>),
    /// Unlock a bunker upgrade for purchase / installation.
    UnlockUpgrade(Id<Upgrade>),
    /// Grant the player an intel entry. No-op if already known.
    GiveIntel(Id<Intel>),
    /// Spawn a visitor from the given NPC template, optionally
    /// at a specific area. `at` = `None` defers to the template's
    /// default spawn location (bunker visitor queue today; may
    /// be a random area in the future).
    SpawnNpc {
        template: Id<NpcTemplate>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<Id<Area>>,
    },
    /// Grant the player experience. Rank is derived from total
    /// XP. Wraps [`Experience`] so the types line up with
    /// `PlayerState::xp` and the intent is explicit at the call
    /// site.
    GivePlayerXp(Experience),
    /// Grant an NPC template experience. The sim resolves the
    /// template to one live instance (e.g. the quest's current
    /// giver) at apply time.
    GiveNpcXp {
        template: Id<NpcTemplate>,
        amount: Experience,
    },
    /// Shift the danger rating of an area, or the whole zone if
    /// [`area`](Consequence::DangerModifier::area) is `None`. A
    /// non-`None` [`duration`](Consequence::DangerModifier::duration)
    /// schedules automatic expiry; permanent when omitted.
    DangerModifier {
        area: Option<Id<Area>>,
        delta: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<Duration>,
    },
    /// Multiply market prices for an item category. Stacks
    /// multiplicatively with other active modifiers. A non-`None`
    /// [`duration`](Consequence::PriceModifier::duration) schedules
    /// automatic expiry; permanent when omitted.
    PriceModifier {
        category: ItemCategory,
        multiplier: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<Duration>,
    },
}
