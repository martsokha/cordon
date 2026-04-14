//! Per-frame quest driving and the [`QuestEngineCtx`] system param.
//!
//! The drive half of the engine: a mutable slice of the world
//! that [`drive_active_quests`] runs against each frame.
//! Responsible for time-limit expiry, objective condition
//! evaluation, silent branch forks, and terminal outcome
//! application. This is the only system that mutates player +
//! event state on quest completion, so it owns the full
//! `WorldMut` applier surface.
//!
//! `Talk` stages are deliberately not touched here — they need
//! the dialogue runner that lives in cordon-bevy. The Yarn
//! bridge calls [`advance_after_talk`](super::talk::advance_after_talk)
//! when Yarn returns a choice.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{GameTime, Id};
use cordon_core::world::narrative::{Consequence, Quest, QuestStage, QuestStageKind};
use cordon_data::catalog::GameData;
use cordon_data::gamedata::GameDataResource;

use super::super::condition::WorldView;
use super::super::consequence::{
    GiveNpcXpRequest, SpawnNpcRequest, StartQuestRequest, WorldMut, apply,
};
use super::super::state::{CompletedQuest, QuestLog};
use crate::quest::registry::TemplateRegistry;
use crate::resources::{EventLog, GameClock, Player};

/// Mutable bundle used by [`drive_active_quests`] — the only
/// system that may mutate player + event state, because it
/// runs the consequence applier when a quest reaches an
/// `Outcome` stage. Separate from
/// [`QuestDispatchCtx`](super::dispatch::QuestDispatchCtx) so
/// the dispatchers can keep read-only access to player/events
/// and run in parallel with other read-only systems.
///
/// Also carries the faction index (for
/// [`spawn_event_instance`](crate::day::world_events::spawn_event_instance))
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
    pub registry: Res<'w, TemplateRegistry>,
    pub start_quest_tx: MessageWriter<'w, StartQuestRequest>,
    pub spawn_npc_tx: MessageWriter<'w, SpawnNpcRequest>,
    pub give_npc_xp_tx: MessageWriter<'w, GiveNpcXpRequest>,
}

/// Drive every active quest that is currently on an `Objective`
/// or `Branch` stage, expire any quests past their top-level
/// time limit, and apply + complete any quest that has reached
/// an `Outcome` stage.
///
/// `Talk` stages are *not* touched — the Yarn bridge owns them.
/// `Outcome` stages are collected here and applied afterwards to
/// avoid holding aliasing borrows across the apply step.
pub fn drive_active_quests(mut ctx: QuestEngineCtx, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    let now = ctx.clock.0;
    let catalog = &ctx.data.0;
    let faction_pool: Vec<Id<Faction>> = ctx.factions.0.iter().map(|(id, _)| id.clone()).collect();

    // --- 1. Quest-wide time limits.
    // A quest whose elapsed time exceeds `time_limit` jumps
    // straight to its failure Outcome (the first Outcome stage
    // with `success: false`), so the applier picks it up below
    // like any other completion. Quests without a failure
    // stage or without a time limit are left alone.
    let timed_out: Vec<(Id<Quest>, Id<QuestStage>)> = ctx
        .log
        .active
        .iter()
        .filter_map(|active| {
            let def = catalog.quests.get(&active.def_id)?;
            let limit = def.time_limit?;
            if now.minutes_since(active.started_at) < limit.minutes() {
                return None;
            }
            let fail_stage = def
                .stages
                .iter()
                .find(|s| matches!(&s.kind, QuestStageKind::Outcome(o) if !o.success))?;
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
    let objective_transitions = collect_objective_transitions(
        &ctx.log,
        &ctx.player.0,
        &ctx.events.0,
        &ctx.registry,
        catalog,
        now,
    );
    for (index, next_stage) in objective_transitions {
        if let Some(active) = ctx.log.active.get_mut(index) {
            active.advance_to(next_stage, now);
        }
    }

    // --- 3. Branch stages: pick the first eligible arm or
    // fall through. Same collect-then-apply split so the
    // evaluator's immutable borrow of `log` is released before
    // the mutable advance. Run in the same frame as entry so
    // Branch behaves as a silent fork, not a wait state.
    let branch_transitions = collect_branch_transitions(
        &ctx.log,
        &ctx.player.0,
        &ctx.events.0,
        &ctx.registry,
        catalog,
        now,
    );
    for (index, next_stage) in branch_transitions {
        if let Some(active) = ctx.log.active.get_mut(index) {
            active.advance_to(next_stage, now);
        }
    }

    // --- 4. Outcome stages: apply consequences and complete.
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
            matches!(stage.kind, QuestStageKind::Outcome(_)).then(|| active.def_id.clone())
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
            &ctx.registry,
            &faction_pool,
            &mut rng,
            &mut ctx.start_quest_tx,
            &mut ctx.spawn_npc_tx,
            &mut ctx.give_npc_xp_tx,
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
    registry: &TemplateRegistry,
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
        let QuestStageKind::Objective(obj) = &stage.kind else {
            continue;
        };

        let elapsed = now.minutes_since(active.stage_started_at);
        let timed_out = obj
            .timeout
            .map(|limit| elapsed >= limit.minutes())
            .unwrap_or(false);

        // Per-iteration view: each active quest has its own
        // stage clock, so `Wait { duration }` inside a composite
        // condition reads the right elapsed time.
        let view = WorldView {
            player,
            events,
            quests: log,
            registry,
            now,
            stage_started_at: Some(active.stage_started_at),
        };

        if view.evaluate(&obj.condition) {
            out.push((index, obj.on_success.clone()));
        } else if timed_out {
            match &obj.on_failure {
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
                        .find(|s| matches!(&s.kind, QuestStageKind::Outcome(o) if !o.success))
                    {
                        out.push((index, fail_stage.id.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Walk the active list and decide which quests currently on a
/// `Branch` stage should advance this frame. The first arm
/// whose [`when`](cordon_core::world::narrative::BranchArm::when)
/// evaluates true wins; if nothing matches, the stage's
/// `fallback` is taken. Branch transitions are evaluated on
/// entry and take effect the same frame — Branch never waits.
fn collect_branch_transitions(
    log: &QuestLog,
    player: &cordon_core::entity::player::PlayerState,
    events: &[cordon_core::world::narrative::ActiveEvent],
    registry: &TemplateRegistry,
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
        let QuestStageKind::Branch(br) = &stage.kind else {
            continue;
        };

        let view = WorldView {
            player,
            events,
            quests: log,
            registry,
            now,
            stage_started_at: Some(active.stage_started_at),
        };

        let next = br
            .arms
            .iter()
            .find(|arm| view.evaluate(&arm.when))
            .map(|arm| arm.next_stage.clone())
            .unwrap_or_else(|| br.fallback.clone());
        out.push((index, next));
    }
    out
}

/// Apply an `Outcome` stage's consequences, then move the
/// quest record from `active` to `completed`. Looks the quest
/// up by [`def_id`](Id<Quest>) rather than by `Vec` index so
/// repeated calls (or any concurrent mutation of `log.active`)
/// cannot silently target the wrong entry.
#[allow(clippy::too_many_arguments)]
fn complete_quest(
    log: &mut QuestLog,
    catalog: &GameData,
    def_id: &Id<Quest>,
    now: GameTime,
    player: &mut cordon_core::entity::player::PlayerState,
    events: &mut Vec<cordon_core::world::narrative::ActiveEvent>,
    registry: &TemplateRegistry,
    faction_pool: &[Id<Faction>],
    rng: &mut WyRand,
    start_quest_tx: &mut MessageWriter<StartQuestRequest>,
    spawn_npc_tx: &mut MessageWriter<SpawnNpcRequest>,
    give_npc_xp_tx: &mut MessageWriter<GiveNpcXpRequest>,
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
    let QuestStageKind::Outcome(out) = &stage.kind else {
        return;
    };
    let success = out.success;
    let outcome_stage = active.current_stage.clone();
    let started_at = active.started_at;
    let stage_started_at = active.stage_started_at;
    let flags = active.flags.clone();
    let bundles = out.consequences.clone();

    // First pass: decide which conditional bundles are eligible
    // by evaluating their guards against the live world view.
    // Done before touching the mutable `WorldMut` so the guard
    // evaluation can borrow `log` + `events` immutably.
    let eligible: Vec<&Vec<Consequence>> = {
        let view = WorldView {
            player,
            events,
            quests: log,
            registry,
            now,
            stage_started_at: Some(stage_started_at),
        };
        bundles
            .iter()
            .filter(|b| {
                b.when
                    .as_ref()
                    .map(|cond| view.evaluate(cond))
                    .unwrap_or(true)
            })
            .map(|b| &b.apply)
            .collect()
    };
    // Flatten the eligible bundles into a single consequence
    // list the applier can walk without re-checking guards.
    let to_apply: Vec<Consequence> = eligible.into_iter().flatten().cloned().collect();

    // Second pass: apply. Mutable `WorldMut` now, immutable
    // borrows from the eligibility pass are released.
    let mut world = WorldMut {
        player,
        events,
        data: catalog,
        registry,
        now,
        rng,
        faction_pool,
    };
    for consequence in &to_apply {
        apply(
            consequence,
            &mut world,
            start_quest_tx,
            spawn_npc_tx,
            give_npc_xp_tx,
        );
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
