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
use cordon_core::world::narrative::QuestStageKind;
use cordon_data::gamedata::GameDataResource;

use crate::components::TemplateId;
use crate::death::NpcDied;
use crate::quest::state::QuestLog;
use crate::resources::GameClock;

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
        // Collect (def_id, target_stage) pairs first so we don't
        // hold an immutable borrow of `log` while advancing.
        let mut transitions = Vec::new();
        for active in &log.active {
            let Some(def) = catalog.quests.get(&active.def_id) else {
                continue;
            };
            let Some(stage) = def.stage(&active.current_stage) else {
                continue;
            };
            let QuestStageKind::Talk {
                npc: Some(template),
                on_failure,
                ..
            } = &stage.kind
            else {
                continue;
            };
            if template != &tid.0 {
                continue;
            }
            match on_failure {
                Some(target) => {
                    info!(
                        "quest `{}`: template `{}` died in transit — failing Talk stage `{}` → `{}`",
                        active.def_id.as_str(),
                        tid.0.as_str(),
                        active.current_stage.as_str(),
                        target.as_str()
                    );
                    transitions.push((active.def_id.clone(), target.clone()));
                }
                None => {
                    warn!(
                        "quest `{}`: template `{}` died in transit but Talk stage `{}` has no on_failure — quest will stall",
                        active.def_id.as_str(),
                        tid.0.as_str(),
                        active.current_stage.as_str()
                    );
                }
            }
        }
        for (def_id, target) in transitions {
            if let Some(active) = log.active_instance_mut(&def_id) {
                active.advance_to(target, now);
            }
        }
    }
}
