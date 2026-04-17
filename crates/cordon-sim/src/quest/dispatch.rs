//! Quest trigger dispatch: start quests from events, day
//! rollovers, conditions, and explicit requests.

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Event, Quest, QuestTriggerKind};

use super::context::QuestCtx;
use super::messages::StartQuestRequest;
use crate::day::DayRolled;

/// Drain [`StartQuestRequest`]s from consequence application.
/// Uses individual params instead of [`QuestCtx`] because the
/// context's `MessageWriter<StartQuestRequest>` would conflict
/// with the `MessageReader` here.
pub fn process_start_quest_requests(
    mut log: ResMut<super::state::QuestLog>,
    data: Res<cordon_data::gamedata::GameDataResource>,
    clock: Res<crate::resources::GameClock>,
    mut requests: MessageReader<StartQuestRequest>,
) {
    let now = clock.0;
    let quests: Vec<Id<Quest>> = requests.read().map(|req| req.quest.clone()).collect();
    for quest in quests {
        let Some(def) = data.0.quests.get(&quest) else {
            warn!("start_quest: unknown quest `{}`", quest.as_str());
            continue;
        };
        if !def.repeatable {
            if log.is_active(&quest) {
                continue;
            }
            if log.completed.iter().any(|c| c.def_id == quest && c.success) {
                continue;
            }
        }
        let Some(entry) = def.entry_stage() else {
            warn!("start_quest: quest `{}` has no stages", quest.as_str());
            continue;
        };
        let active = super::state::ActiveQuest::new(quest.clone(), entry.id.clone(), now);
        log.active.push(active);
        info!("quest `{}` started (via request)", quest.as_str());
    }
}

/// Fire `OnGameStart` triggers once.
pub fn dispatch_on_game_start(mut ctx: QuestCtx) {
    let now = ctx.now();
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

/// Fire `OnDay` triggers when the day rolls over.
pub fn dispatch_on_day(mut ctx: QuestCtx, mut rolled: MessageReader<DayRolled>) {
    let now = ctx.now();
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

/// Fire `OnEvent` triggers for newly-active events.
pub fn dispatch_on_event(mut ctx: QuestCtx, mut previous: Local<HashSet<Id<Event>>>) {
    let now = ctx.now();
    let current: HashSet<_> = ctx.events.0.iter().map(|e| e.def_id.clone()).collect();
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

/// Fire `OnCondition` triggers on rising edge.
pub fn dispatch_on_condition(
    mut ctx: QuestCtx,
    mut previously_eligible: Local<HashSet<Id<cordon_core::world::narrative::QuestTrigger>>>,
) {
    let now = ctx.now();
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

    let mut eligible_now = HashSet::new();
    let mut to_fire = Vec::new();

    for (trigger, cond) in &triggers {
        if !ctx.evaluate(cond, None) {
            continue;
        }
        if let Some(req) = &trigger.requires {
            if !ctx.evaluate(req, None) {
                continue;
            }
        }
        eligible_now.insert(trigger.id.clone());
        if !previously_eligible.contains(&trigger.id) {
            to_fire.push(trigger.clone());
        }
    }
    *previously_eligible = eligible_now;

    for trigger in to_fire {
        ctx.try_fire_trigger(&trigger, now);
    }
}
