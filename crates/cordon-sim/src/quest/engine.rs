//! Quest state transitions driven by world state.
//!
//! This module covers the parts of the quest lifecycle that are
//! self-contained within the sim layer:
//!
//! - **Trigger dispatch** — watch [`QuestTriggerDef`]s and push
//!   matching ones into [`QuestLog::active`] via the shared
//!   [`start_quest`] helper.
//! - **Objective driving** — every frame, evaluate the current
//!   `Objective` stage's condition and advance on success /
//!   timeout.
//! - **Outcome application** — when a quest enters an `Outcome`
//!   stage, apply its consequences and move it to `completed`.
//!
//! `Talk` stages are *not* driven here. They need to speak to
//! the dialogue runner, which lives in cordon-bevy. The bridge
//! lives there and only calls back into [`advance_after_talk`]
//! when Yarn returns a choice.

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::quest::{
    Quest, QuestDef, QuestStage, QuestStageKind, QuestTriggerKind,
};
use cordon_data::catalog::GameData;
use cordon_data::gamedata::GameDataResource;

use super::condition::{WorldView, evaluate};
use super::consequence::{StartQuestRequest, WorldMut, apply};
use super::state::{ActiveQuest, CompletedQuest, QuestLog};
use crate::day::DayRolled;
use crate::resources::{EventLog, GameClock, Player};

/// Begin a new instance of `quest` if one isn't already active
/// and the quest isn't marked non-repeatable + already completed.
/// Returns the index of the newly-started quest within
/// `log.active`, or `None` when the start was suppressed.
pub fn start_quest(
    log: &mut QuestLog,
    data: &GameData,
    quest: &Id<Quest>,
    now: GameTime,
) -> Option<usize> {
    let Some(def) = data.quests.get(quest) else {
        warn!("start_quest: unknown quest `{}`", quest.as_str());
        return None;
    };
    if !def.repeatable {
        if log.is_active(quest) {
            return None;
        }
        if log
            .completed
            .iter()
            .any(|c| &c.def_id == quest && c.success)
        {
            return None;
        }
    }
    let Some(entry) = def.stages.first() else {
        warn!(
            "start_quest: quest `{}` has no stages, skipping",
            quest.as_str()
        );
        return None;
    };
    let active = ActiveQuest::new(quest.clone(), entry.id.clone(), now);
    log.active.push(active);
    info!("quest `{}` started", quest.as_str());
    Some(log.active.len() - 1)
}

/// After a Yarn dialogue tied to a `Talk` stage finishes, jump
/// to the branch whose `choice` matches the supplied value, or
/// to the stage's `fallback` if nothing matches. Call this from
/// the cordon-bevy dialogue bridge.
pub fn advance_after_talk(
    log: &mut QuestLog,
    data: &GameData,
    quest: &Id<Quest>,
    choice: Option<&str>,
    now: GameTime,
) {
    let Some(active) = log.active_instance_mut(quest) else {
        return;
    };
    let Some(def) = data.quests.get(&active.def_id) else {
        return;
    };
    let Some(stage) = def.stages.iter().find(|s| s.id == active.current_stage) else {
        return;
    };
    let QuestStageKind::Talk {
        branches, fallback, ..
    } = &stage.kind
    else {
        return;
    };
    let next = match choice {
        Some(c) => branches
            .iter()
            .find(|b| b.choice == c)
            .map(|b| b.next_stage.clone())
            .unwrap_or_else(|| fallback.clone()),
        None => fallback.clone(),
    };
    active.advance_to(next, now);
}

/// Drive every active quest that is currently on an `Objective`
/// stage: evaluate the condition, advance on success, jump to
/// the failure stage (or fail the quest outright) on timeout.
///
/// `Talk` stages are *not* touched — the Yarn bridge owns them.
/// `Outcome` stages are collected here and applied afterwards to
/// avoid holding aliasing borrows across the apply step.
pub fn drive_active_quests(
    mut log: ResMut<QuestLog>,
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut player: ResMut<Player>,
    mut events: ResMut<EventLog>,
    mut start_quest_tx: MessageWriter<StartQuestRequest>,
) {
    let now = clock.0;
    let catalog = &data.0;

    // --- 1. Objective stages: condition + timeout handling.
    // We must not mutate `log` while evaluating a condition that
    // also borrows `log`. Collect the transitions first, apply
    // them in a second pass.
    let objective_transitions =
        collect_objective_transitions(&log, &player.0, &events.0, catalog, now);
    for (index, next_stage) in objective_transitions {
        if let Some(active) = log.active.get_mut(index) {
            active.advance_to(next_stage, now);
        }
    }

    // --- 2. Outcome stages: apply consequences and complete.
    // Collect indices of quests whose current stage is an
    // Outcome (anything parked there this frame should finish).
    let mut to_complete: Vec<usize> = Vec::new();
    for (index, active) in log.active.iter().enumerate() {
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        if let Some(stage) = def.stages.iter().find(|s| s.id == active.current_stage)
            && matches!(stage.kind, QuestStageKind::Outcome { .. })
        {
            to_complete.push(index);
        }
    }
    // Pop in reverse so swap_remove indices stay valid.
    for index in to_complete.into_iter().rev() {
        complete_quest(
            &mut log,
            catalog,
            index,
            now,
            &mut player.0,
            &mut events.0,
            &mut start_quest_tx,
        );
    }
}

/// Walk the active list and decide which quests should
/// transition out of their objective stage this frame. Pure
/// read — returns `(index, next_stage_id)` pairs.
fn collect_objective_transitions(
    log: &QuestLog,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::event::ActiveEvent],
    catalog: &GameData,
    now: GameTime,
) -> Vec<(usize, Id<QuestStage>)> {
    let view = WorldView {
        player,
        events,
        quests: log,
    };
    let mut out = Vec::new();
    for (index, active) in log.active.iter().enumerate() {
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        let Some(stage) = def.stages.iter().find(|s| s.id == active.current_stage) else {
            continue;
        };
        let QuestStageKind::Objective {
            condition,
            timeout_minutes,
            on_success,
            on_failure,
        } = &stage.kind
        else {
            continue;
        };

        let elapsed = now.minutes_since(active.stage_started_at);
        let timed_out = timeout_minutes
            .map(|limit| elapsed >= limit)
            .unwrap_or(false);

        if evaluate(condition, &view) {
            out.push((index, on_success.clone()));
        } else if timed_out {
            match on_failure {
                Some(stage) => out.push((index, stage.clone())),
                None => {
                    // No failure stage: jump to a synthetic
                    // outcome by picking the first Outcome
                    // stage with `success: false`. If the
                    // quest has none, leave it in place — the
                    // authoring is malformed.
                    if let Some(fail_stage) = def
                        .stages
                        .iter()
                        .find(|s| matches!(s.kind, QuestStageKind::Outcome { success: false, .. }))
                    {
                        out.push((index, fail_stage.id.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Apply an Outcome stage's consequences, move the quest record
/// from `active` to `completed`, and drop it out of the active
/// list. `index` is the position in `log.active`.
fn complete_quest(
    log: &mut QuestLog,
    catalog: &GameData,
    index: usize,
    now: GameTime,
    player: &mut cordon_core::entity::player::PlayerState,
    events: &mut Vec<cordon_core::world::event::ActiveEvent>,
    start_quest_tx: &mut MessageWriter<StartQuestRequest>,
) {
    let Some(active) = log.active.get(index) else {
        return;
    };
    let Some(def) = catalog.quests.get(&active.def_id) else {
        return;
    };
    let Some(stage) = def.stages.iter().find(|s| s.id == active.current_stage) else {
        return;
    };
    let QuestStageKind::Outcome {
        success,
        consequences,
    } = &stage.kind
    else {
        return;
    };
    let success = *success;
    let outcome_stage = active.current_stage.clone();
    let started_at = active.started_at;
    let def_id = active.def_id.clone();
    let flags = active.flags.clone();

    // Apply before moving records so consequences can reference
    // the still-active quest (e.g. chained `StartQuest`).
    let mut world = WorldMut {
        player,
        events,
        data: catalog,
        now,
    };
    for consequence in consequences {
        apply(consequence, &mut world, start_quest_tx);
    }

    log.active.swap_remove(index);
    log.completed.push(CompletedQuest {
        def_id: def_id.clone(),
        started_at,
        completed_at: now,
        success,
        outcome_stage,
        flags,
    });
    info!(
        "quest `{}` completed ({})",
        def_id.as_str(),
        if success { "success" } else { "failure" }
    );
}

/// Drain [`StartQuestRequest`]s produced by consequence
/// application and turn them into fresh `ActiveQuest` entries.
/// Runs after [`drive_active_quests`] each frame.
pub fn process_start_quest_requests(
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut requests: MessageReader<StartQuestRequest>,
) {
    let now = clock.0;
    for req in requests.read() {
        start_quest(&mut log, &data.0, &req.quest, now);
    }
}

/// Fire `OnGameStart` triggers once, on the first frame the
/// sim is fully bootstrapped. Scheduled with
/// `.run_if(resource_added::<GameClock>)` so it runs exactly
/// once — [`GameClock`] is inserted by `init_world_resources`
/// on `OnEnter(AppState::Playing)`, after `GameDataResource`
/// is already live, so all sim state is ready by then.
pub fn dispatch_on_game_start(
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    player: Res<Player>,
    events: Res<EventLog>,
) {
    let now = clock.0;
    let catalog = &data.0;
    let triggers: Vec<_> = catalog
        .triggers
        .values()
        .filter(|t| matches!(t.kind, QuestTriggerKind::OnGameStart))
        .cloned()
        .collect();
    for trigger in triggers {
        try_fire_trigger(&mut log, catalog, &trigger, now, &player.0, &events.0);
    }
}

/// Fire `OnDay` triggers whose target day matches the new day.
pub fn dispatch_on_day(
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    player: Res<Player>,
    events: Res<EventLog>,
    mut rolled: MessageReader<DayRolled>,
) {
    for ev in rolled.read() {
        let catalog = &data.0;
        let now = clock.0;
        let triggers: Vec<_> = catalog
            .triggers
            .values()
            .filter(|t| matches!(t.kind, QuestTriggerKind::OnDay(d) if d == ev.new_day))
            .cloned()
            .collect();
        for trigger in triggers {
            try_fire_trigger(&mut log, catalog, &trigger, now, &player.0, &events.0);
        }
    }
}

/// Evaluate `extra_requires` against world state and, if the
/// trigger is eligible, start its quest. Handles the
/// repeat-guard via [`QuestLog::fired_triggers`].
fn try_fire_trigger(
    log: &mut QuestLog,
    catalog: &GameData,
    trigger: &cordon_core::world::narrative::quest::QuestTriggerDef,
    now: GameTime,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::event::ActiveEvent],
) {
    if !trigger.repeatable && log.fired_triggers.contains(&trigger.id) {
        return;
    }
    // Scope the immutable borrow of `log` so the mutable
    // `start_quest` call below is free of aliasing. The
    // eligibility check is pure, so this split is just a borrow
    // shuffle — behaviour is identical to a single pass.
    let eligible = {
        let view = WorldView {
            player,
            events,
            quests: log,
        };
        trigger.extra_requires.iter().all(|c| evaluate(c, &view))
    };
    if !eligible {
        return;
    }
    if start_quest(log, catalog, &trigger.quest, now).is_some() {
        log.fired_triggers.insert(trigger.id.clone());
    }
}

/// Minimal type-check that the quest def exists for triggers
/// loaded at startup. Warns about dangling references so
/// authoring errors surface without crashing the sim.
///
/// Scheduled with `.run_if(resource_added::<GameDataResource>)`
/// so it runs exactly once, on the frame the catalog first
/// appears. No `Local<bool>` guard needed — Bevy's resource
/// change detection handles the "fire once" semantic natively.
pub fn validate_trigger_references(data: Res<GameDataResource>) {
    let catalog = &data.0;
    for trigger in catalog.triggers.values() {
        if !catalog.quests.contains_key(&trigger.quest) {
            warn!(
                "quest trigger `{}` references unknown quest `{}`",
                trigger.id.as_str(),
                trigger.quest.as_str()
            );
        }
    }
    // Also sanity-check that every quest has at least one stage.
    for def in catalog.quests.values() {
        if def.stages.is_empty() {
            warn!("quest `{}` has no stages", def.id.as_str());
        }
        validate_stage_references(def);
    }
}

fn validate_stage_references(def: &QuestDef) {
    let ids: HashSet<_> = def.stages.iter().map(|s| &s.id).collect();
    for stage in &def.stages {
        match &stage.kind {
            QuestStageKind::Talk {
                branches, fallback, ..
            } => {
                if !ids.contains(fallback) {
                    warn!(
                        "quest `{}` stage `{}` has unknown fallback `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        fallback.as_str()
                    );
                }
                for branch in branches {
                    if !ids.contains(&branch.next_stage) {
                        warn!(
                            "quest `{}` stage `{}` branch `{}` → unknown stage `{}`",
                            def.id.as_str(),
                            stage.id.as_str(),
                            branch.choice,
                            branch.next_stage.as_str()
                        );
                    }
                }
            }
            QuestStageKind::Objective {
                on_success,
                on_failure,
                ..
            } => {
                if !ids.contains(on_success) {
                    warn!(
                        "quest `{}` stage `{}` on_success → unknown stage `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        on_success.as_str()
                    );
                }
                if let Some(on_failure) = on_failure
                    && !ids.contains(on_failure)
                {
                    warn!(
                        "quest `{}` stage `{}` on_failure → unknown stage `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        on_failure.as_str()
                    );
                }
            }
            QuestStageKind::Outcome { .. } => {}
        }
    }
}
