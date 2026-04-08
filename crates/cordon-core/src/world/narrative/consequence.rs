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

use super::quest::Quest;
use crate::entity::bunker::Upgrade;
use crate::entity::faction::Faction;
use crate::entity::npc::NpcTemplate;
use crate::item::{Item, ItemCategory, StashScope};
use crate::primitive::{Credits, Id, Relation};
use crate::world::area::Area;
use crate::world::event::Event;

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
    /// the scoped stash(es).
    HaveItem {
        item: Id<Item>,
        #[serde(default = "default_item_count")]
        count: u32,
        #[serde(default)]
        scope: StashScope,
    },
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
    /// The given event is currently active.
    EventActive(Id<Event>),
    /// The given quest is currently active.
    QuestActive(Id<Quest>),
    /// The given quest has been completed successfully.
    QuestCompleted(Id<Quest>),
    /// A flag on the given active quest equals a specific string
    /// value. For numeric / boolean flags the evaluator coerces
    /// via Yarn's value cast rules.
    QuestFlag {
        quest: Id<Quest>,
        key: String,
        equals: String,
    },
    /// Trivial condition — always true. Used with a stage
    /// `timeout_minutes` to implement "wait N minutes then
    /// advance" without any world dependency.
    Wait,
    /// All of the nested conditions must be true.
    AllOf(Vec<ObjectiveCondition>),
    /// At least one of the nested conditions must be true.
    AnyOf(Vec<ObjectiveCondition>),
    /// Logical negation of the nested condition.
    Not(Box<ObjectiveCondition>),
}

fn default_item_count() -> u32 {
    1
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
        delta: Relation,
    },
    /// Credit the player with currency.
    GiveCredits(Credits),
    /// Debit the player's currency.
    TakeCredits(Credits),
    /// Place an item into the player's stash in the given scope.
    GiveItem {
        item: Id<Item>,
        #[serde(default)]
        scope: StashScope,
    },
    /// Remove an item from the player's stash in the given scope.
    TakeItem {
        item: Id<Item>,
        #[serde(default)]
        scope: StashScope,
    },
    /// Fire an event by its definition ID.
    TriggerEvent(Id<Event>),
    /// Start a quest manually (bypassing its trigger table).
    StartQuest(Id<Quest>),
    /// Unlock a bunker upgrade for purchase / installation.
    UnlockUpgrade(Id<Upgrade>),
    /// Spawn a visitor from the given NPC template.
    SpawnNpc(Id<NpcTemplate>),
    /// Grant the player experience. Rank is derived from total XP.
    GivePlayerXp(u32),
    /// Grant an NPC template experience. The sim resolves the
    /// template to one live instance (e.g. the quest's current
    /// giver) at apply time.
    GiveNpcXp {
        template: Id<NpcTemplate>,
        amount: u32,
    },
    /// Shift the danger rating of an area, or the whole zone if
    /// [`area`](Consequence::DangerModifier::area) is `None`.
    DangerModifier { area: Option<Id<Area>>, delta: f32 },
    /// Multiply market prices for an item category. Stacks
    /// multiplicatively with other active modifiers.
    PriceModifier {
        category: ItemCategory,
        multiplier: f32,
    },
}
