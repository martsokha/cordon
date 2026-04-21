//! Per-frame quest driving: time-limit expiry, objective/branch
//! stage transitions, outcome application, and talk completion.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Consequence, Quest, QuestStage, QuestStageKind};

use super::context::QuestCtx;
use super::messages::{QuestFinished, QuestUpdated, TalkCompleted};
use super::state::CompletedQuest;

/// Drive all active quests: expire timed-out quests, evaluate
/// objectives and branches, apply outcomes.
pub fn drive_active_quests(mut ctx: QuestCtx, mut rng: Single<&mut WyRand, With<GlobalRng>>) {
    let now = ctx.now();
    let catalog = &ctx.data.0;
    let faction_pool = ctx.faction_pool();

    // 1. Quest-wide time limits → jump to failure outcome.
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
                "quest `{}` timed out → `{}`",
                quest_id.as_str(),
                fail_stage.as_str()
            );
            active.advance_to(fail_stage, now);
            ctx.quest_updated_tx.write(QuestUpdated { quest: quest_id });
        }
    }

    // 2. Objective stages: collect transitions immutably, apply mutably.
    let objective_transitions = collect_objective_transitions(&ctx);
    for (index, next_stage) in objective_transitions {
        if let Some(active) = ctx.log.active.get_mut(index) {
            let quest = active.def_id.clone();
            active.advance_to(next_stage, now);
            ctx.quest_updated_tx.write(QuestUpdated { quest });
        }
    }

    // 3. Branch stages: pick first eligible arm or fallback.
    let branch_transitions: Vec<(usize, Id<QuestStage>)> = {
        let catalog = &ctx.data.0;
        ctx.log
            .active
            .iter()
            .enumerate()
            .filter_map(|(index, active)| {
                let def = catalog.quests.get(&active.def_id)?;
                let stage = def.stage(&active.current_stage)?;
                let QuestStageKind::Branch(br) = &stage.kind else {
                    return None;
                };
                let next = br
                    .arms
                    .iter()
                    .find(|arm| ctx.evaluate(&arm.when, Some(active.stage_started_at)))
                    .map(|arm| arm.next_stage.clone())
                    .unwrap_or_else(|| br.fallback.clone());
                Some((index, next))
            })
            .collect()
    };
    for (index, next_stage) in branch_transitions {
        if let Some(active) = ctx.log.active.get_mut(index) {
            active.advance_to(next_stage, now);
        }
    }

    // 4. Outcome stages: apply consequences and complete.
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
        complete_quest(&mut ctx, &def_id, &faction_pool, &mut rng);
    }
}

/// Handle [`TalkCompleted`] messages from the Yarn bridge.
pub fn handle_talk_completed(mut ctx: QuestCtx, mut completed: MessageReader<TalkCompleted>) {
    let now = ctx.now();
    let catalog = &ctx.data.0;

    for ev in completed.read() {
        let next = {
            let Some(active) = ctx.log.active_instance(&ev.quest) else {
                continue;
            };
            let Some(def) = catalog.quests.get(&active.def_id) else {
                continue;
            };
            let Some(stage) = def.stage(&active.current_stage) else {
                continue;
            };
            let QuestStageKind::Talk(talk) = &stage.kind else {
                continue;
            };
            match &ev.choice {
                Some(c) => talk
                    .branches
                    .iter()
                    .filter(|b| {
                        b.requires
                            .as_ref()
                            .map(|cond| ctx.evaluate(cond, Some(active.stage_started_at)))
                            .unwrap_or(true)
                    })
                    .find(|b| b.choice == *c)
                    .map(|b| b.next_stage.clone())
                    .unwrap_or_else(|| talk.fallback.clone()),
                None => talk.fallback.clone(),
            }
        };
        if let Some(active) = ctx.log.active_instance_mut(&ev.quest) {
            active.advance_to(next, now);
            ctx.quest_updated_tx.write(QuestUpdated {
                quest: ev.quest.clone(),
            });
        }
    }
}

fn collect_objective_transitions(ctx: &QuestCtx) -> Vec<(usize, Id<QuestStage>)> {
    let catalog = &ctx.data.0;
    let now = ctx.now();
    let mut out = Vec::new();
    for (index, active) in ctx.log.active.iter().enumerate() {
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

        if ctx.evaluate(&obj.condition, Some(active.stage_started_at)) {
            out.push((index, obj.on_success.clone()));
        } else if timed_out {
            match &obj.on_failure {
                Some(stage) => out.push((index, stage.clone())),
                None => {
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

/// Apply an Outcome stage's consequences and move the quest to
/// completed.
fn complete_quest(
    ctx: &mut QuestCtx,
    def_id: &Id<Quest>,
    faction_pool: &[Id<Faction>],
    rng: &mut WyRand,
) {
    let Some(active) = ctx.log.active_instance(def_id) else {
        return;
    };
    let catalog = &ctx.data.0;
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
    let now = ctx.now();

    // Phase 1: evaluate guards (immutable).
    let to_apply: Vec<Consequence> = bundles
        .iter()
        .filter(|b| {
            b.when
                .as_ref()
                .map(|cond| ctx.evaluate(cond, Some(stage_started_at)))
                .unwrap_or(true)
        })
        .flat_map(|b| b.apply.iter().cloned())
        .collect();

    // Phase 2: apply consequences (mutable).
    for c in &to_apply {
        ctx.apply_consequence(c, faction_pool, rng);
    }

    ctx.log.active.retain(|a| &a.def_id != def_id);
    ctx.log.completed.push(CompletedQuest {
        def_id: def_id.clone(),
        started_at,
        completed_at: now,
        success,
        outcome_stage,
        flags,
    });
    ctx.quest_finished_tx.write(QuestFinished {
        quest: def_id.clone(),
        success,
    });
    info!(
        "quest `{}` completed ({})",
        def_id.as_str(),
        if success { "success" } else { "failure" }
    );
}
