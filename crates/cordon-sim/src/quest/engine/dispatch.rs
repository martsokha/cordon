//! Trigger dispatch and the [`QuestDispatchCtx`] system param.
//!
//! The dispatch half of the engine: a read-only slice of the
//! world (+ mutable quest log) that every trigger dispatcher
//! and the start-quest request processor runs against.
//! [`QuestDispatchCtx`] owns the two mutation primitives —
//! [`start_quest`](QuestDispatchCtx::start_quest) and
//! [`try_fire_trigger`](QuestDispatchCtx::try_fire_trigger) —
//! so every dispatcher shares a single entry point without
//! rewiring arguments on each call.
//!
//! `dispatch_on_*` systems are thin wrappers that filter the
//! trigger table by [`QuestTriggerKind`] and forward eligible
//! triggers into [`try_fire_trigger`](QuestDispatchCtx::try_fire_trigger).

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{
    Event, Quest, QuestTrigger, QuestTriggerDef, QuestTriggerKind,
};
use cordon_data::gamedata::GameDataResource;

use super::super::condition::WorldView;
use super::super::consequence::StartQuestRequest;
use super::super::state::{ActiveQuest, QuestLog};
use crate::day::DayRolled;
use crate::resources::{EventLog, GameClock, Player};

/// Read-only bundle for quest-dispatch systems.
///
/// Every trigger dispatcher reads the same world slice —
/// quest log, catalog, clock, player, events — and mutates
/// only the quest log. Bundling them as a derive
/// [`SystemParam`] keeps the parameter list stable across
/// dispatchers and avoids `ResMut<Player>` / `ResMut<EventLog>`
/// claims that would block parallelism with other systems.
#[derive(SystemParam)]
pub struct QuestDispatchCtx<'w> {
    pub log: ResMut<'w, QuestLog>,
    pub data: Res<'w, GameDataResource>,
    pub clock: Res<'w, GameClock>,
    pub player: Res<'w, Player>,
    pub events: Res<'w, EventLog>,
}

impl<'w> QuestDispatchCtx<'w> {
    /// Begin a new instance of `quest` if one isn't already
    /// active and the quest isn't marked non-repeatable +
    /// already completed. Returns the index of the newly-started
    /// quest within `log.active`, or `None` when the start was
    /// suppressed.
    pub fn start_quest(&mut self, quest: &Id<Quest>, now: GameTime) -> Option<usize> {
        let Some(def) = self.data.0.quests.get(quest) else {
            warn!("start_quest: unknown quest `{}`", quest.as_str());
            return None;
        };
        if !def.repeatable {
            if self.log.is_active(quest) {
                return None;
            }
            if self
                .log
                .completed
                .iter()
                .any(|c| &c.def_id == quest && c.success)
            {
                return None;
            }
        }
        let Some(entry) = def.entry_stage() else {
            warn!(
                "start_quest: quest `{}` has no stages, skipping",
                quest.as_str()
            );
            return None;
        };
        let active = ActiveQuest::new(quest.clone(), entry.id.clone(), now);
        self.log.active.push(active);
        info!("quest `{}` started", quest.as_str());
        Some(self.log.active.len() - 1)
    }

    /// Evaluate a trigger's `requires` clause against world
    /// state and, if eligible, [`start_quest`](Self::start_quest)
    /// its target. Handles the repeat-guard via
    /// [`QuestLog::fired_triggers`].
    pub fn try_fire_trigger(&mut self, trigger: &QuestTriggerDef, now: GameTime) {
        if !trigger.repeatable && self.log.fired_triggers.contains(&trigger.id) {
            return;
        }
        // Scope the immutable borrow of `log` so the mutable
        // `start_quest` call below is free of aliasing. The
        // eligibility check is pure, so this split is just a
        // borrow shuffle — behaviour is identical to a single
        // pass.
        let eligible = match &trigger.requires {
            None => true,
            Some(cond) => {
                let view = WorldView {
                    player: &self.player.0,
                    events: &self.events.0,
                    quests: &self.log,
                    now,
                    stage_started_at: None,
                };
                view.evaluate(cond)
            }
        };
        if !eligible {
            return;
        }
        if self.start_quest(&trigger.quest, now).is_some() {
            self.log.fired_triggers.insert(trigger.id.clone());
        }
    }
}

/// Drain [`StartQuestRequest`]s produced by consequence
/// application and turn them into fresh `ActiveQuest` entries.
/// Runs after [`drive_active_quests`](super::drive_active_quests)
/// each frame.
pub fn process_start_quest_requests(
    mut ctx: QuestDispatchCtx,
    mut requests: MessageReader<StartQuestRequest>,
) {
    let now = ctx.clock.0;
    // Collect first to release the message reader borrow before
    // the mutable `ctx.start_quest` call — start_quest writes to
    // ctx.log and doesn't touch the reader, but decoupling makes
    // the borrow structure obvious.
    let quests: Vec<Id<Quest>> = requests.read().map(|req| req.quest.clone()).collect();
    for quest in quests {
        ctx.start_quest(&quest, now);
    }
}

/// Fire `OnGameStart` triggers once, on the first frame the
/// sim is fully bootstrapped. Scheduled with
/// `.run_if(resource_added::<GameClock>)` so it runs exactly
/// once — [`GameClock`] is inserted by `init_world_resources`
/// on `OnEnter(AppState::Playing)`, after `GameDataResource`
/// is already live, so all sim state is ready by then.
pub fn dispatch_on_game_start(mut ctx: QuestDispatchCtx) {
    let now = ctx.clock.0;
    // Clone the matching trigger list out of the catalog before
    // the mutable `try_fire_trigger` call so the data borrow is
    // released. Same pattern for every `dispatch_on_*` system.
    let triggers: Vec<_> = ctx
        .data
        .0
        .triggers
        .values()
        .filter(|t| matches!(t.kind, QuestTriggerKind::OnGameStart))
        .cloned()
        .collect();
    for trigger in triggers {
        ctx.try_fire_trigger(&trigger, now);
    }
}

/// Fire `OnDay` triggers whose target day matches the new day.
pub fn dispatch_on_day(mut ctx: QuestDispatchCtx, mut rolled: MessageReader<DayRolled>) {
    let now = ctx.clock.0;
    // Collect the day values so the reader borrow is released
    // before we touch `ctx` mutably inside try_fire_trigger.
    let new_days: Vec<_> = rolled.read().map(|ev| ev.new_day).collect();
    for day in new_days {
        let triggers: Vec<_> = ctx
            .data
            .0
            .triggers
            .values()
            .filter(|t| matches!(t.kind, QuestTriggerKind::OnDay(d) if d == day))
            .cloned()
            .collect();
        for trigger in triggers {
            ctx.try_fire_trigger(&trigger, now);
        }
    }
}

/// Fire `OnEvent` triggers for events that *just became
/// active*. Diffs the current [`EventLog`] against a local
/// snapshot of def IDs seen last frame; any new ID fires
/// every trigger whose [`OnEvent`](QuestTriggerKind::OnEvent)
/// discriminant matches it.
///
/// Using a def-ID snapshot (rather than the `ActiveEvent`
/// objects themselves) means re-triggering for multiple
/// instances of the same event is intentionally skipped —
/// quest triggers are about kind-level "this has started
/// happening in the world", not instance counts.
pub fn dispatch_on_event(mut ctx: QuestDispatchCtx, mut previous: Local<HashSet<Id<Event>>>) {
    let now = ctx.clock.0;
    let current: HashSet<_> = ctx.events.0.iter().map(|e| e.def_id.clone()).collect();
    // Newly-active events are in `current` but not `previous`.
    let new_events: Vec<_> = current.difference(&*previous).cloned().collect();
    *previous = current;

    for event_id in new_events {
        let triggers: Vec<_> = ctx
            .data
            .0
            .triggers
            .values()
            .filter(|t| matches!(&t.kind, QuestTriggerKind::OnEvent(id) if id == &event_id))
            .cloned()
            .collect();
        for trigger in triggers {
            ctx.try_fire_trigger(&trigger, now);
        }
    }
}

/// Fire `OnCondition` triggers every frame, on the rising
/// edge of their condition. A `Local<HashSet<Id<QuestTrigger>>>`
/// of triggers whose condition was `true` on the previous
/// frame suppresses re-firing while the condition remains
/// true — without this, a trigger gated on
/// `FactionStanding { min_standing: Neutral }` would fire
/// every single frame for the entire game.
///
/// Non-repeatable triggers additionally latch via
/// [`QuestLog::fired_triggers`] inside
/// [`try_fire_trigger`](QuestDispatchCtx::try_fire_trigger),
/// so this rising-edge mechanism matters most for repeatable
/// condition triggers.
pub fn dispatch_on_condition(
    mut ctx: QuestDispatchCtx,
    mut previously_true: Local<HashSet<Id<QuestTrigger>>>,
) {
    let now = ctx.clock.0;
    // Evaluate every OnCondition trigger. The eligibility list
    // is computed first (immutable borrow of `ctx.log` inside
    // the view), then the rising-edge set is fired mutably.
    let triggers: Vec<_> = ctx
        .data
        .0
        .triggers
        .values()
        .filter_map(|t| match &t.kind {
            QuestTriggerKind::OnCondition(cond) => Some((t.clone(), cond.clone())),
            _ => None,
        })
        .collect();

    let mut now_true: HashSet<Id<QuestTrigger>> = HashSet::new();
    let mut to_fire: Vec<_> = Vec::new();
    {
        let view = WorldView {
            player: &ctx.player.0,
            events: &ctx.events.0,
            quests: &ctx.log,
            now,
            // Trigger-requires has no per-stage clock; `Wait`
            // in a trigger is meaningless and the evaluator
            // will warn if it appears.
            stage_started_at: None,
        };
        for (trigger, cond) in &triggers {
            if view.evaluate(cond) {
                now_true.insert(trigger.id.clone());
                // Rising edge only: fire if the condition
                // was not true last frame.
                if !previously_true.contains(&trigger.id) {
                    to_fire.push(trigger.clone());
                }
            }
        }
    }
    *previously_true = now_true;

    for trigger in to_fire {
        ctx.try_fire_trigger(&trigger, now);
    }
}
