//! Grouping structs for the [`condition::evaluate`](super::condition::evaluate)
//! and [`consequence::apply`](super::consequence::apply) parameter lists.
//!
//! Three concerns, two visibilities (read-only vs. mutating):
//!
//! | concern          | shared ref       | mutable ref         |
//! |------------------|------------------|---------------------|
//! | player state     | [`PlayerView`]   | [`PlayerRefs`]      |
//! | sim inputs       | [`SimView`]      | [`SimRefs`]         |
//! | outgoing events  | —                | [`QuestTx`]         |
//!
//! The evaluator needs only the `*View` shared-ref variants; the
//! applier needs all three mutating variants plus the explicit
//! `rng` / `faction_pool` params.
//!
//! Callers in Bevy systems build these via [`QuestCtx`](super::context::QuestCtx)
//! methods so the `SystemParam` assembly happens in one place.
//! Unit tests build them by hand.

use bevy::prelude::MessageWriter;
use cordon_core::primitive::GameTime;
use cordon_core::world::narrative::ActiveEvent;
use cordon_data::catalog::GameData;

use super::messages::{
    DecisionRecorded, EndGameRequest, GiveNpcXpRequest, SpawnNpcRequest, StandingChanged,
    StartQuestRequest,
};
use super::registry::TemplateRegistry;
use super::state::QuestLog;
use crate::bunker::pills::PlayerPills;
use crate::resources::{
    PlayerDecisions, PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash, PlayerUpgrades,
};

/// Shared view over the player-owned resources the condition
/// evaluator reads.
pub struct PlayerView<'a> {
    pub identity: &'a PlayerIdentity,
    pub standings: &'a PlayerStandings,
    pub upgrades: &'a PlayerUpgrades,
    pub stash: &'a PlayerStash,
    pub intel: &'a PlayerIntel,
    pub decisions: &'a PlayerDecisions,
}

/// Mutating view over the same player resources — used by the
/// consequence applier.
pub struct PlayerRefs<'a> {
    pub identity: &'a mut PlayerIdentity,
    pub standings: &'a mut PlayerStandings,
    pub upgrades: &'a mut PlayerUpgrades,
    pub stash: &'a mut PlayerStash,
    pub intel: &'a mut PlayerIntel,
    pub decisions: &'a mut PlayerDecisions,
}

impl<'a> PlayerRefs<'a> {
    /// Reborrow as a shared view for condition evaluation. Lets the
    /// applier feed its own mut-refs into an evaluator call without
    /// giving up the outer `&mut` scope.
    pub fn as_view(&self) -> PlayerView<'_> {
        PlayerView {
            identity: self.identity,
            standings: self.standings,
            upgrades: self.upgrades,
            stash: self.stash,
            intel: self.intel,
            decisions: self.decisions,
        }
    }
}

/// Shared view over the read-only sim resources the evaluator uses.
pub struct SimView<'a> {
    pub data: &'a GameData,
    pub registry: &'a TemplateRegistry,
    pub quests: &'a QuestLog,
    pub pills: &'a PlayerPills,
    pub events: &'a [ActiveEvent],
    pub now: GameTime,
}

/// Mutating view over the sim resources — only the event log is
/// actually mutated (by [`Consequence::TriggerEvent`](cordon_core::world::narrative::Consequence::TriggerEvent)).
/// The other fields are shared refs because the applier doesn't
/// touch them.
pub struct SimRefs<'a> {
    pub data: &'a GameData,
    pub registry: &'a TemplateRegistry,
    pub quests: &'a QuestLog,
    pub pills: &'a PlayerPills,
    pub events: &'a mut Vec<ActiveEvent>,
    pub now: GameTime,
}

impl<'a> SimRefs<'a> {
    /// Reborrow as a shared view for condition evaluation.
    pub fn as_view(&self) -> SimView<'_> {
        SimView {
            data: self.data,
            registry: self.registry,
            quests: self.quests,
            pills: self.pills,
            events: self.events,
            now: self.now,
        }
    }
}

/// Outgoing message writers fired by consequence application.
/// Quest-lifecycle senders (`quest_started_tx` etc.) live on
/// [`QuestCtx`](super::context::QuestCtx) directly — only the
/// consequence-fired ones are grouped here.
pub struct QuestTx<'a, 'w> {
    pub start_quest: &'a mut MessageWriter<'w, StartQuestRequest>,
    pub spawn_npc: &'a mut MessageWriter<'w, SpawnNpcRequest>,
    pub give_npc_xp: &'a mut MessageWriter<'w, GiveNpcXpRequest>,
    pub standing_changed: &'a mut MessageWriter<'w, StandingChanged>,
    pub decision_recorded: &'a mut MessageWriter<'w, DecisionRecorded>,
    pub end_game: &'a mut MessageWriter<'w, EndGameRequest>,
}
