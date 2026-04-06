//! Consequences and conditions shared by quests and events.
//!
//! [`Consequence`] describes what happens when a quest stage completes,
//! a choice is made, or an event fires. [`ObjectiveCondition`] describes
//! what must be true for a quest objective to succeed or a choice to
//! be available.

use serde::{Deserialize, Serialize};

use crate::entity::bunker::Upgrade;
use crate::entity::faction::Faction;
use crate::entity::npc::{Npc, NpcTemplate};
use crate::item::ItemCategory;
use crate::item::def::Item;
use crate::primitive::credits::Credits;
use crate::primitive::id::Id;
use crate::primitive::relation::Relation;
use crate::primitive::uid::Uid;
use crate::world::area::Area;
use crate::world::event::Event;
use crate::world::narrative::quest::Quest;

/// A condition that must be met.
///
/// Used for quest objectives (what the player must do) and for
/// gating choice availability (choice only appears if condition is met).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveCondition {
    /// Player must have a specific item in storage.
    HaveItem(Id<Item>),
    /// Player must have at least this many credits.
    HaveCredits(Credits),
    /// Player must reach a minimum standing with a faction.
    FactionStanding {
        faction: Id<Faction>,
        min_standing: Relation,
    },
    /// Player must have a specific upgrade installed.
    HaveUpgrade(Id<Upgrade>),
    /// A specific event must be active in the world.
    EventActive(Id<Event>),
    /// A specific quest must be currently active.
    QuestActive(Id<Quest>),
    /// A specific quest must have been completed successfully.
    QuestCompleted(Id<Quest>),
    /// Player must deliver a specific item to the quest NPC.
    DeliverItem(Id<Item>),
    /// Simply wait (used with timeout_days on the stage).
    Wait,
}

/// A consequence applied when a choice is made, a quest stage
/// completes, or an event fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Consequence {
    /// Change standing with a faction.
    StandingChange {
        faction: Id<Faction>,
        delta: Relation,
    },
    /// Give credits to the player.
    GiveCredits(Credits),
    /// Take credits from the player.
    TakeCredits(Credits),
    /// Give an item to the player (placed in storage).
    GiveItem(Id<Item>),
    /// Remove an item from the player's storage.
    TakeItem(Id<Item>),
    /// Trigger an event by its def ID.
    TriggerEvent(Id<Event>),
    /// Start a quest.
    StartQuest(Id<Quest>),
    /// Unlock an upgrade (make it available for purchase/installation).
    UnlockUpgrade(Id<Upgrade>),
    /// Spawn a named NPC visitor (references an NPC template ID from config).
    SpawnNpc(Id<NpcTemplate>),
    /// Immediately fail the current quest (only meaningful in quest context).
    FailQuest,
    /// Award experience points to the player. Rank is derived from
    /// accumulated XP — enough XP triggers an automatic rank-up.
    GivePlayerXp(u32),
    /// Award experience points to an NPC (by runtime UID). NPC rank
    /// is derived from accumulated XP.
    GiveNpcXp(Uid<Npc>, u32),
    /// Modify danger in a target area.
    DangerModifier {
        /// Area ID. If `None`, applies zone-wide.
        area: Option<Id<Area>>,
        /// Additive danger change.
        delta: f32,
    },
    /// Modify market prices for an item category.
    PriceModifier {
        /// Item category affected.
        category: ItemCategory,
        /// Price multiplier (e.g., 1.5 = 50% more expensive).
        multiplier: f32,
    },
}
