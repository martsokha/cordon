//! Quest-side handling for template NPC deaths during travel.
//!
//! When a template NPC tagged with [`TemplateId`] dies while any
//! active quest is parked on a `Talk` stage that waits for that
//! same template, the stage is transitioned to the Talk stage's
//! [`on_failure`](cordon_core::world::narrative::QuestStageKind::Talk)
//! target. Quests whose Talk stage has no `on_failure` stall —
//! the death is logged and the quest is left alone, matching
//! the behaviour documented on the field.

use bevy::prelude::*;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Quest, QuestStage, QuestStageKind};
use cordon_data::catalog::GameData;
use cordon_data::gamedata::GameDataResource;

use super::state::QuestLog;
use crate::behavior::death::NpcDied;
use crate::entity::npc::TemplateId;
use crate::resources::GameClock;

/// Outcome of planning a template death against the quest log.
///
/// - `Fail(target)` — the quest's Talk stage is waiting on this
///   template AND authored an on_failure target; caller should
///   advance to that target.
/// - `Stall` — Talk stage is waiting but no on_failure authored;
///   the quest will stall until content is fixed.
/// - `Ignore` — quest is not currently parked on a Talk stage
///   that references this template (advanced past it, on a
///   different stage kind, or a different template).
#[derive(Debug, PartialEq, Eq)]
pub enum TalkDeathOutcome {
    Fail(Id<QuestStage>),
    Stall,
    Ignore,
}

/// Pure logic: given a quest definition, its current stage id,
/// and a dying template id, decide what to do. Extracted from
/// [`fail_talk_on_template_death`] so it's unit-testable without
/// spinning up a Bevy [`App`].
pub fn plan_talk_death(
    quest_def: &cordon_core::world::narrative::QuestDef,
    current_stage_id: &Id<QuestStage>,
    dead_template: &Id<NpcTemplate>,
) -> TalkDeathOutcome {
    let Some(stage) = quest_def.stage(current_stage_id) else {
        return TalkDeathOutcome::Ignore;
    };
    let QuestStageKind::Talk(talk) = &stage.kind else {
        return TalkDeathOutcome::Ignore;
    };
    let Some(template) = &talk.npc else {
        return TalkDeathOutcome::Ignore;
    };
    if template != dead_template {
        return TalkDeathOutcome::Ignore;
    }
    match &talk.on_failure {
        Some(target) => TalkDeathOutcome::Fail(target.clone()),
        None => TalkDeathOutcome::Stall,
    }
}

/// Scan all active quests for Talk stages that were waiting on
/// the dying template, returning the list of `(quest_id, target)`
/// transitions to apply. Pure function — no ECS access — so
/// callers can unit-test the planning independently of the
/// Bevy-driven system.
pub fn plan_all_talk_deaths(
    catalog: &GameData,
    log: &QuestLog,
    dead_template: &Id<NpcTemplate>,
) -> Vec<(Id<Quest>, Id<QuestStage>)> {
    let mut transitions = Vec::new();
    for active in &log.active {
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        match plan_talk_death(def, &active.current_stage, dead_template) {
            TalkDeathOutcome::Fail(target) => {
                transitions.push((active.def_id.clone(), target));
            }
            TalkDeathOutcome::Stall => {
                warn!(
                    "quest `{}`: template `{}` died in transit but Talk stage `{}` has no on_failure — quest will stall",
                    active.def_id.as_str(),
                    dead_template.as_str(),
                    active.current_stage.as_str()
                );
            }
            TalkDeathOutcome::Ignore => {}
        }
    }
    transitions
}

/// Watch for [`NpcDied`] on template NPCs and fail any active
/// quest whose current Talk stage was waiting on that template.
pub fn fail_talk_on_template_death(
    mut deaths: MessageReader<NpcDied>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    template_q: Query<&TemplateId>,
    mut log: ResMut<QuestLog>,
) {
    let catalog = &data.0;
    let now = clock.0;
    for ev in deaths.read() {
        let Ok(tid) = template_q.get(ev.entity) else {
            continue;
        };
        let transitions = plan_all_talk_deaths(catalog, &log, &tid.0);
        for (def_id, target) in transitions {
            info!(
                "quest `{}`: template `{}` died in transit — failing to `{}`",
                def_id.as_str(),
                tid.0.as_str(),
                target.as_str()
            );
            if let Some(active) = log.active_instance_mut(&def_id) {
                active.advance_to(target, now);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cordon_core::primitive::GameTime;
    use cordon_core::world::narrative::{
        OutcomeStage, QuestCategory, QuestDef, QuestStageDef, QuestStageKind, TalkBranch, TalkStage,
    };

    use super::*;
    use crate::quest::state::ActiveQuest;

    fn template(s: &str) -> Id<NpcTemplate> {
        Id::<NpcTemplate>::new(s)
    }

    fn quest_with_talk(
        talk_npc: Option<Id<NpcTemplate>>,
        on_failure: Option<Id<QuestStage>>,
    ) -> QuestDef {
        QuestDef {
            id: Id::new("q_test"),
            category: QuestCategory::Side,
            giver: None,
            giver_faction: None,
            time_limit: None,
            stages: vec![QuestStageDef {
                id: Id::new("intro"),
                kind: QuestStageKind::Talk(TalkStage {
                    npc: talk_npc,
                    yarn_node: "q_test.intro".to_string(),
                    branches: vec![TalkBranch {
                        choice: "accept".to_string(),
                        next_stage: Id::new("done"),
                        requires: None,
                    }],
                    fallback: Id::new("done"),
                    on_failure,
                }),
            }],
            repeatable: false,
        }
    }

    #[test]
    fn plan_fails_when_template_matches_and_on_failure_set() {
        let q = quest_with_talk(Some(template("npc_lt")), Some(Id::new("failed")));
        let outcome = plan_talk_death(&q, &Id::new("intro"), &template("npc_lt"));
        assert_eq!(outcome, TalkDeathOutcome::Fail(Id::new("failed")));
    }

    #[test]
    fn plan_stalls_when_template_matches_but_no_on_failure() {
        let q = quest_with_talk(Some(template("npc_lt")), None);
        let outcome = plan_talk_death(&q, &Id::new("intro"), &template("npc_lt"));
        assert_eq!(outcome, TalkDeathOutcome::Stall);
    }

    #[test]
    fn plan_ignores_when_template_does_not_match() {
        let q = quest_with_talk(Some(template("npc_lt")), Some(Id::new("failed")));
        let outcome = plan_talk_death(&q, &Id::new("intro"), &template("npc_fixer"));
        assert_eq!(outcome, TalkDeathOutcome::Ignore);
    }

    #[test]
    fn plan_ignores_when_talk_has_no_npc() {
        let q = quest_with_talk(None, Some(Id::new("failed")));
        let outcome = plan_talk_death(&q, &Id::new("intro"), &template("npc_lt"));
        assert_eq!(outcome, TalkDeathOutcome::Ignore);
    }

    #[test]
    fn plan_ignores_when_current_stage_is_not_talk() {
        let mut q = quest_with_talk(Some(template("npc_lt")), Some(Id::new("failed")));
        q.stages[0].kind = QuestStageKind::Outcome(OutcomeStage {
            success: true,
            consequences: vec![],
        });
        let outcome = plan_talk_death(&q, &Id::new("intro"), &template("npc_lt"));
        assert_eq!(outcome, TalkDeathOutcome::Ignore);
    }

    #[test]
    fn plan_ignores_when_stage_id_missing_from_def() {
        let q = quest_with_talk(Some(template("npc_lt")), Some(Id::new("failed")));
        let outcome = plan_talk_death(&q, &Id::new("nonexistent"), &template("npc_lt"));
        assert_eq!(outcome, TalkDeathOutcome::Ignore);
    }

    #[test]
    fn plan_all_walks_active_quest_log() {
        let q = quest_with_talk(Some(template("npc_lt")), Some(Id::new("failed")));
        let mut catalog = GameData::default();
        catalog.quests.insert(q.id.clone(), q);
        let log = QuestLog {
            active: vec![ActiveQuest {
                def_id: Id::new("q_test"),
                current_stage: Id::new("intro"),
                started_at: GameTime::new(),
                stage_started_at: GameTime::new(),
                flags: Default::default(),
            }],
            completed: vec![],
            fired_triggers: Default::default(),
        };
        let transitions = plan_all_talk_deaths(&catalog, &log, &template("npc_lt"));
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].0, Id::new("q_test"));
        assert_eq!(transitions[0].1, Id::new("failed"));
    }

    #[test]
    fn plan_all_empty_when_log_has_no_active_quests() {
        let catalog = GameData::default();
        let log = QuestLog::default();
        let transitions = plan_all_talk_deaths(&catalog, &log, &template("npc_lt"));
        assert!(transitions.is_empty());
    }
}
