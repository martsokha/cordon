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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{
    Consequence, Event, Quest, QuestDef, QuestStage, QuestStageKind, QuestTrigger, QuestTriggerKind,
};
use cordon_data::catalog::GameData;
use cordon_data::gamedata::GameDataResource;

use super::condition::{WorldView, evaluate};
use super::consequence::{StartQuestRequest, WorldMut, apply};
use super::state::{ActiveQuest, CompletedQuest, QuestLog};

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

/// Mutable bundle used by [`drive_active_quests`] — the only
/// system that may mutate player + event state, because it
/// runs the consequence applier when a quest reaches an
/// `Outcome` stage. Separate from [`QuestDispatchCtx`] so the
/// dispatchers can keep read-only access to player/events and
/// run in parallel with other read-only systems.
///
/// Also carries the faction index (for
/// [`spawn_event_instance`](crate::day::events::spawn_event_instance))
/// so consequence-driven event fires can roll real random
/// instances instead of hardcoding def-minimum values.
#[derive(SystemParam)]
pub struct QuestEngineCtx<'w> {
    pub log: ResMut<'w, QuestLog>,
    pub data: Res<'w, GameDataResource>,
    pub clock: Res<'w, GameClock>,
    pub player: ResMut<'w, Player>,
    pub events: ResMut<'w, EventLog>,
    pub factions: Res<'w, crate::resources::FactionIndex>,
    pub start_quest_tx: MessageWriter<'w, StartQuestRequest>,
}
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
    let Some(entry) = def.entry_stage() else {
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
    let Some(stage) = def.stage(&active.current_stage) else {
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
pub fn drive_active_quests(mut ctx: QuestEngineCtx, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    let now = ctx.clock.0;
    let catalog = &ctx.data.0;
    let faction_pool: Vec<Id<Faction>> = ctx.factions.0.iter().map(|(id, _)| id.clone()).collect();

    // --- 1. Quest-wide time limits.
    // A quest whose elapsed time exceeds `time_limit_minutes`
    // jumps straight to its failure Outcome (the first Outcome
    // stage with `success: false`), so the applier picks it
    // up below like any other completion. Quests without a
    // failure stage or without a time limit are left alone.
    let timed_out: Vec<(Id<Quest>, Id<QuestStage>)> = ctx
        .log
        .active
        .iter()
        .filter_map(|active| {
            let def = catalog.quests.get(&active.def_id)?;
            let limit = def.time_limit_minutes?;
            if now.minutes_since(active.started_at) < limit {
                return None;
            }
            let fail_stage = def
                .stages
                .iter()
                .find(|s| matches!(s.kind, QuestStageKind::Outcome { success: false, .. }))?;
            Some((active.def_id.clone(), fail_stage.id.clone()))
        })
        .collect();
    for (quest_id, fail_stage) in timed_out {
        if let Some(active) = ctx.log.active_instance_mut(&quest_id) {
            info!(
                "quest `{}` exceeded time limit, jumping to `{}`",
                quest_id.as_str(),
                fail_stage.as_str()
            );
            active.advance_to(fail_stage, now);
        }
    }

    // --- 2. Objective stages: condition + timeout handling.
    // We must not mutate `log` while evaluating a condition that
    // also borrows `log`. Collect the transitions first, apply
    // them in a second pass.
    let objective_transitions =
        collect_objective_transitions(&ctx.log, &ctx.player.0, &ctx.events.0, catalog, now);
    for (index, next_stage) in objective_transitions {
        if let Some(active) = ctx.log.active.get_mut(index) {
            active.advance_to(next_stage, now);
        }
    }

    // --- 3. Outcome stages: apply consequences and complete.
    // Collect the def_ids of quests whose current stage is an
    // Outcome, then resolve each by def_id at completion time.
    // Identifying by def_id rather than by index survives any
    // mid-apply mutation of `log.active` (e.g. chained quest
    // starts through the message channel don't disturb the
    // resolution — though in practice StartQuest is deferred
    // to a later system, so this is defence in depth).
    let to_complete: Vec<Id<Quest>> = ctx
        .log
        .active
        .iter()
        .filter_map(|active| {
            let def = catalog.quests.get(&active.def_id)?;
            let stage = def.stage(&active.current_stage)?;
            matches!(stage.kind, QuestStageKind::Outcome { .. }).then(|| active.def_id.clone())
        })
        .collect();
    for def_id in to_complete {
        complete_quest(
            &mut ctx.log,
            catalog,
            &def_id,
            now,
            &mut ctx.player.0,
            &mut ctx.events.0,
            &faction_pool,
            &mut rng,
            &mut ctx.start_quest_tx,
        );
    }
}

/// Walk the active list and decide which quests should
/// transition out of their objective stage this frame. Pure
/// read — returns `(index, next_stage_id)` pairs.
fn collect_objective_transitions(
    log: &QuestLog,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::narrative::ActiveEvent],
    catalog: &GameData,
    now: GameTime,
) -> Vec<(usize, Id<QuestStage>)> {
    let mut out = Vec::new();
    for (index, active) in log.active.iter().enumerate() {
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        let Some(stage) = def.stage(&active.current_stage) else {
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

        // Per-iteration view: each active quest has its own
        // stage clock, so `Wait { duration }` inside a composite
        // condition reads the right elapsed time.
        let view = WorldView {
            player,
            events,
            quests: log,
            now,
            stage_started_at: Some(active.stage_started_at),
        };

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

/// Apply an `Outcome` stage's consequences, then move the
/// quest record from `active` to `completed`. Looks the quest
/// up by [`def_id`](Id<Quest>) rather than by `Vec` index so
/// repeated calls (or any concurrent mutation of
/// `log.active`) cannot silently target the wrong entry.
fn complete_quest(
    log: &mut QuestLog,
    catalog: &GameData,
    def_id: &Id<Quest>,
    now: GameTime,
    player: &mut cordon_core::entity::player::PlayerState,
    events: &mut Vec<cordon_core::world::narrative::ActiveEvent>,
    faction_pool: &[Id<Faction>],
    rng: &mut WyRand,
    start_quest_tx: &mut MessageWriter<StartQuestRequest>,
) {
    // Resolve the active instance by def_id. Cloning the
    // scalar state we need here lets us drop the borrow of
    // `log` before the mutable applier runs below.
    let Some(active) = log.active_instance(def_id) else {
        return;
    };
    let Some(def) = catalog.quests.get(def_id) else {
        return;
    };
    let Some(stage) = def.stage(&active.current_stage) else {
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
    let flags = active.flags.clone();
    let consequences = consequences.clone();

    // Apply before moving records so consequences can reference
    // the still-active quest (e.g. chained `StartQuest`).
    let mut world = WorldMut {
        player,
        events,
        data: catalog,
        now,
        rng,
        faction_pool,
    };
    for consequence in &consequences {
        apply(consequence, &mut world, start_quest_tx);
    }

    // Stable removal: retain() walks once, evicts the matching
    // entry, and leaves every other active quest in its
    // original order. `swap_remove` was correct for the
    // reverse-index iteration pattern used previously but was
    // fragile — def_id + retain is obviously right.
    log.active.retain(|a| &a.def_id != def_id);
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
    mut ctx: QuestDispatchCtx,
    mut requests: MessageReader<StartQuestRequest>,
) {
    let now = ctx.clock.0;
    for req in requests.read() {
        start_quest(&mut ctx.log, &ctx.data.0, &req.quest, now);
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
    let catalog = &ctx.data.0;
    let triggers: Vec<_> = catalog
        .triggers
        .values()
        .filter(|t| matches!(t.kind, QuestTriggerKind::OnGameStart))
        .cloned()
        .collect();
    for trigger in triggers {
        try_fire_trigger(
            &mut ctx.log,
            catalog,
            &trigger,
            now,
            &ctx.player.0,
            &ctx.events.0,
        );
    }
}

/// Fire `OnDay` triggers whose target day matches the new day.
pub fn dispatch_on_day(mut ctx: QuestDispatchCtx, mut rolled: MessageReader<DayRolled>) {
    let now = ctx.clock.0;
    let catalog = &ctx.data.0;
    for ev in rolled.read() {
        let triggers: Vec<_> = catalog
            .triggers
            .values()
            .filter(|t| matches!(t.kind, QuestTriggerKind::OnDay(d) if d == ev.new_day))
            .cloned()
            .collect();
        for trigger in triggers {
            try_fire_trigger(
                &mut ctx.log,
                catalog,
                &trigger,
                now,
                &ctx.player.0,
                &ctx.events.0,
            );
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
    let catalog = &ctx.data.0;
    let now = ctx.clock.0;
    let current: HashSet<_> = ctx.events.0.iter().map(|e| e.def_id.clone()).collect();
    // Newly-active events are in `current` but not `previous`.
    let new_events: Vec<_> = current.difference(&*previous).cloned().collect();
    *previous = current;

    for event_id in new_events {
        let triggers: Vec<_> = catalog
            .triggers
            .values()
            .filter(|t| matches!(&t.kind, QuestTriggerKind::OnEvent(id) if id == &event_id))
            .cloned()
            .collect();
        for trigger in triggers {
            try_fire_trigger(
                &mut ctx.log,
                catalog,
                &trigger,
                now,
                &ctx.player.0,
                &ctx.events.0,
            );
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
/// [`QuestLog::fired_triggers`] inside [`try_fire_trigger`],
/// so this rising-edge mechanism matters most for
/// repeatable condition triggers.
pub fn dispatch_on_condition(
    mut ctx: QuestDispatchCtx,
    mut previously_true: Local<HashSet<Id<QuestTrigger>>>,
) {
    let catalog = &ctx.data.0;
    let now = ctx.clock.0;
    // Evaluate every OnCondition trigger. This is an
    // immutable borrow of `log` inside the view, so the
    // eligibility list is computed before any mutations.
    let triggers: Vec<_> = catalog
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
            if evaluate(cond, &view) {
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
        try_fire_trigger(
            &mut ctx.log,
            catalog,
            &trigger,
            now,
            &ctx.player.0,
            &ctx.events.0,
        );
    }
}

/// Evaluate a trigger's [`requires`](QuestTriggerDef::requires)
/// clause against world state and, if eligible, start its
/// quest. Handles the repeat-guard via
/// [`QuestLog::fired_triggers`].
fn try_fire_trigger(
    log: &mut QuestLog,
    catalog: &GameData,
    trigger: &cordon_core::world::narrative::QuestTriggerDef,
    now: GameTime,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::narrative::ActiveEvent],
) {
    if !trigger.repeatable && log.fired_triggers.contains(&trigger.id) {
        return;
    }
    // Scope the immutable borrow of `log` so the mutable
    // `start_quest` call below is free of aliasing. The
    // eligibility check is pure, so this split is just a borrow
    // shuffle — behaviour is identical to a single pass.
    let eligible = match &trigger.requires {
        None => true,
        Some(cond) => {
            let view = WorldView {
                player,
                events,
                quests: log,
                now,
                stage_started_at: None,
            };
            evaluate(cond, &view)
        }
    };
    if !eligible {
        return;
    }
    if start_quest(log, catalog, &trigger.quest, now).is_some() {
        log.fired_triggers.insert(trigger.id.clone());
    }
}

/// Minimal type-check that the quest + trigger catalog is
/// internally consistent and that authored content does not
/// rely on consequence variants that are currently stubbed.
///
/// Warns on:
/// - dangling quest references in trigger definitions
/// - quests with zero stages
/// - stage branch / fallback / on_success / on_failure
///   references that don't match any stage ID in the quest
/// - authored consequences that hit the stub path in the
///   applier ([`SpawnNpc`](Consequence::SpawnNpc),
///   [`GiveNpcXp`](Consequence::GiveNpcXp),
///   [`DangerModifier`](Consequence::DangerModifier),
///   [`PriceModifier`](Consequence::PriceModifier))
///
/// Scheduled with `.run_if(resource_added::<GameDataResource>)`
/// so it runs exactly once, on the frame the catalog first
/// appears. No `Local<bool>` guard needed — Bevy's resource
/// change detection handles the "fire once" semantic natively.
pub fn validate_catalog(data: Res<GameDataResource>) {
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
    warn_on_stub_consequences(catalog);
}

/// Walk every consequence in every quest stage and every
/// event definition, counting how many times each currently-
/// stubbed variant appears. Emits one summary warning per
/// stub variant that is actually authored against, so a
/// quest designer sees the problem at game-load time rather
/// than only when the consequence fires at runtime.
fn warn_on_stub_consequences(catalog: &GameData) {
    let mut spawn_npc = 0usize;
    let mut give_npc_xp = 0usize;
    let mut danger_modifier = 0usize;
    let mut price_modifier = 0usize;

    let mut count = |c: &Consequence| match c {
        Consequence::SpawnNpc(_) => spawn_npc += 1,
        Consequence::GiveNpcXp { .. } => give_npc_xp += 1,
        Consequence::DangerModifier { .. } => danger_modifier += 1,
        Consequence::PriceModifier { .. } => price_modifier += 1,
        _ => {}
    };

    for def in catalog.quests.values() {
        for stage in &def.stages {
            let QuestStageKind::Outcome { consequences, .. } = &stage.kind else {
                continue;
            };
            for consequence in consequences {
                count(consequence);
            }
        }
    }
    for event in catalog.events.values() {
        for consequence in &event.consequences {
            count(consequence);
        }
    }

    if spawn_npc > 0 {
        warn!(
            "STUB CONSEQUENCE `spawn_npc` referenced {spawn_npc}× in authored content \
             — no visitor queue bridge yet, these will no-op at runtime."
        );
    }
    if give_npc_xp > 0 {
        warn!(
            "STUB CONSEQUENCE `give_npc_xp` referenced {give_npc_xp}× in authored content \
             — no template→entity resolver yet, these will no-op at runtime."
        );
    }
    if danger_modifier > 0 {
        warn!(
            "STUB CONSEQUENCE `danger_modifier` referenced {danger_modifier}× in authored content \
             — no AreaStates bridge yet, these will no-op at runtime."
        );
    }
    if price_modifier > 0 {
        warn!(
            "STUB CONSEQUENCE `price_modifier` referenced {price_modifier}× in authored content \
             — no market system yet, these will no-op at runtime."
        );
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
