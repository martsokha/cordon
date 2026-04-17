//! Unified quest system parameter.
//!
//! [`QuestCtx`] is the single SystemParam used by all quest
//! systems (dispatch, drive, talk handling). It owns mutable
//! access to all player resources and message writers so there's
//! one construction site, one place to add new fields.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{ObjectiveCondition, Quest, QuestTriggerDef};
use cordon_data::gamedata::GameDataResource;

use super::condition;
use super::messages::{
    GiveNpcXpRequest, QuestFinished, QuestStarted, QuestUpdated, SpawnNpcRequest, StandingChanged,
    StartQuestRequest,
};
use super::registry::TemplateRegistry;
use super::state::QuestLog;
use crate::resources::{
    EventLog, FactionIndex, GameClock, PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash,
    PlayerUpgrades,
};

#[derive(SystemParam)]
pub struct QuestCtx<'w> {
    pub log: ResMut<'w, QuestLog>,
    pub data: Res<'w, GameDataResource>,
    pub clock: Res<'w, GameClock>,
    pub identity: ResMut<'w, PlayerIdentity>,
    pub standings: ResMut<'w, PlayerStandings>,
    pub upgrades: ResMut<'w, PlayerUpgrades>,
    pub stash: ResMut<'w, PlayerStash>,
    pub intel: ResMut<'w, PlayerIntel>,
    pub events: ResMut<'w, EventLog>,
    pub factions: Res<'w, FactionIndex>,
    pub registry: Res<'w, TemplateRegistry>,
    pub start_quest_tx: MessageWriter<'w, StartQuestRequest>,
    pub spawn_npc_tx: MessageWriter<'w, SpawnNpcRequest>,
    pub give_npc_xp_tx: MessageWriter<'w, GiveNpcXpRequest>,
    pub standing_changed_tx: MessageWriter<'w, StandingChanged>,
    pub quest_started_tx: MessageWriter<'w, QuestStarted>,
    pub quest_updated_tx: MessageWriter<'w, QuestUpdated>,
    pub quest_finished_tx: MessageWriter<'w, QuestFinished>,
}

impl QuestCtx<'_> {
    pub fn now(&self) -> GameTime {
        self.clock.0
    }

    /// Evaluate a condition against live world state.
    pub fn evaluate(&self, cond: &ObjectiveCondition, stage_started_at: Option<GameTime>) -> bool {
        condition::evaluate(
            cond,
            &self.identity,
            &self.standings,
            &self.upgrades,
            &self.stash,
            &self.intel,
            &self.events.0,
            &self.log,
            &self.registry,
            self.now(),
            stage_started_at,
        )
    }

    /// Faction IDs extracted from the faction index. Needed by
    /// the consequence applier for event instancing.
    pub fn faction_pool(&self) -> Vec<Id<cordon_core::entity::faction::Faction>> {
        self.factions.0.iter().map(|(id, _)| id.clone()).collect()
    }

    /// Start a quest if eligible. Delegates validation to
    /// [`QuestLog::try_start`].
    pub fn start_quest(&mut self, quest: &Id<Quest>, now: GameTime) -> Option<usize> {
        let Some(def) = self.data.0.quests.get(quest) else {
            warn!("start_quest: unknown quest `{}`", quest.as_str());
            return None;
        };
        let result = self.log.try_start(def, now);
        if result.is_some() {
            self.quest_started_tx
                .write(QuestStarted { quest: quest.clone() });
        }
        result
    }

    /// Evaluate a trigger's `requires` clause and start its quest
    /// if eligible.
    pub fn try_fire_trigger(&mut self, trigger: &QuestTriggerDef, now: GameTime) {
        if !trigger.repeatable && self.log.fired_triggers.contains(&trigger.id) {
            return;
        }
        let eligible = match &trigger.requires {
            None => true,
            Some(cond) => self.evaluate(cond, None),
        };
        if !eligible {
            return;
        }
        if self.start_quest(&trigger.quest, now).is_some() {
            self.log.fired_triggers.insert(trigger.id.clone());
        }
    }
}
