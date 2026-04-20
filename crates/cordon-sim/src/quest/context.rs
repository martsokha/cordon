//! Unified quest system parameter.
//!
//! [`QuestCtx`] is the single SystemParam used by all quest
//! systems (dispatch, drive, talk handling). It owns mutable
//! access to all player resources and message writers so there's
//! one construction site, one place to add new fields.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_prng::WyRand;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{Consequence, ObjectiveCondition, Quest, QuestTriggerDef};
use cordon_data::gamedata::GameDataResource;

use super::messages::{
    DecisionRecorded, EndGameRequest, GiveNpcXpRequest, QuestFinished, QuestStarted, QuestUpdated,
    SpawnNpcRequest, StandingChanged, StartQuestRequest,
};
use super::refs::{PlayerRefs, PlayerView, QuestTx, SimRefs, SimView};
use super::registry::TemplateRegistry;
use super::state::QuestLog;
use super::{condition, consequence};
use crate::bunker::pills::PlayerPills;
use crate::resources::{
    EventLog, FactionIndex, GameClock, PlayerDecisions, PlayerIdentity, PlayerIntel,
    PlayerStandings, PlayerStash, PlayerUpgrades,
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
    pub decisions: ResMut<'w, PlayerDecisions>,
    pub pills: Res<'w, PlayerPills>,
    pub events: ResMut<'w, EventLog>,
    pub factions: Res<'w, FactionIndex>,
    pub registry: Res<'w, TemplateRegistry>,
    pub start_quest_tx: MessageWriter<'w, StartQuestRequest>,
    pub spawn_npc_tx: MessageWriter<'w, SpawnNpcRequest>,
    pub give_npc_xp_tx: MessageWriter<'w, GiveNpcXpRequest>,
    pub standing_changed_tx: MessageWriter<'w, StandingChanged>,
    pub decision_recorded_tx: MessageWriter<'w, DecisionRecorded>,
    pub quest_started_tx: MessageWriter<'w, QuestStarted>,
    pub quest_updated_tx: MessageWriter<'w, QuestUpdated>,
    pub quest_finished_tx: MessageWriter<'w, QuestFinished>,
    pub end_game_tx: MessageWriter<'w, EndGameRequest>,
}

impl QuestCtx<'_> {
    pub fn now(&self) -> GameTime {
        self.clock.0
    }

    /// Evaluate a condition against live world state. Builds
    /// [`PlayerView`] and [`SimView`] — shared-ref variants so the
    /// call site remains `&self` and can coexist with other borrows
    /// of [`QuestCtx`].
    pub fn evaluate(&self, cond: &ObjectiveCondition, stage_started_at: Option<GameTime>) -> bool {
        let players = PlayerView {
            identity: &self.identity,
            standings: &self.standings,
            upgrades: &self.upgrades,
            stash: &self.stash,
            intel: &self.intel,
            decisions: &self.decisions,
        };
        let sim = SimView {
            data: &self.data.0,
            registry: &self.registry,
            quests: &self.log,
            pills: &self.pills,
            events: &self.events.0,
            now: self.now(),
        };
        condition::evaluate(cond, &players, &sim, stage_started_at)
    }

    /// Faction IDs extracted from the faction index. Needed by
    /// the consequence applier for event instancing.
    pub fn faction_pool(&self) -> Vec<Id<Faction>> {
        self.factions.0.iter().map(|(id, _)| id.clone()).collect()
    }

    /// Apply a single consequence to world state. Builds
    /// [`PlayerRefs`], [`QuestTx`], and [`SimCtx`] so the call site
    /// is `ctx.apply_consequence(c, &faction_pool, rng)` instead of
    /// an eighteen-argument free-function invocation.
    pub fn apply_consequence(
        &mut self,
        consequence: &Consequence,
        faction_pool: &[Id<Faction>],
        rng: &mut WyRand,
    ) {
        let now = self.now();
        let mut players = PlayerRefs {
            identity: &mut self.identity,
            standings: &mut self.standings,
            upgrades: &mut self.upgrades,
            stash: &mut self.stash,
            intel: &mut self.intel,
            decisions: &mut self.decisions,
        };
        let mut tx = QuestTx {
            start_quest: &mut self.start_quest_tx,
            spawn_npc: &mut self.spawn_npc_tx,
            give_npc_xp: &mut self.give_npc_xp_tx,
            standing_changed: &mut self.standing_changed_tx,
            decision_recorded: &mut self.decision_recorded_tx,
            end_game: &mut self.end_game_tx,
        };
        let mut sim = SimRefs {
            data: &self.data.0,
            registry: &self.registry,
            quests: &self.log,
            pills: &self.pills,
            events: &mut self.events.0,
            now,
        };
        consequence::apply(
            consequence,
            &mut players,
            &mut tx,
            &mut sim,
            rng,
            faction_pool,
        );
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
            self.quest_started_tx.write(QuestStarted {
                quest: quest.clone(),
            });
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
