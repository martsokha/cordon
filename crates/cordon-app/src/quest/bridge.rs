//! The concrete glue between quest state and the dialogue runner.
//!
//! Two entry points:
//!
//! - [`enqueue_talk_dialogue`] runs every frame in the bunker
//!   state. For each active quest parked on a `Talk` stage
//!   that hasn't been dispatched yet, it either (a) pushes a
//!   [`Visitor`] onto [`VisitorQueue`] when the stage has an
//!   NPC, or (b) fires [`StartDialogue`] directly for
//!   narrator-only stages. The bridge-owned
//!   [`DialogueInFlight`] slot is set at dispatch so the same
//!   stage isn't double-dispatched while a template NPC is
//!   still travelling.
//! - [`on_dialogue_completed`] is a Bevy observer on
//!   `NodeCompleted`. It matches the just-ended yarn node
//!   against the active quests' current Talk stages and only
//!   acts on an exact match — so unrelated dialogue nodes
//!   (e.g. a visitor's trade line that happens to finish
//!   while a quest NPC is travelling) don't advance the
//!   wrong quest.
//!
//! Dialogue is strictly serial: at any moment there is at most
//! one quest's Talk stage dispatched. The visitor queue keeps
//! visitor-driven dialogue serial; narrator-only stages only
//! dispatch when the slot is already empty.

use bevy::prelude::*;
use bevy_yarnspinner::events::NodeCompleted;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Quest, QuestStageKind};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::entity::npc::TemplateId;
use cordon_sim::plugin::prelude::QuestLog;
use cordon_sim::quest::messages::{DismissTemplateNpc, SpawnNpcRequest, TalkCompleted};

use crate::bunker::resources::StartDialogue;
use crate::bunker::{Visitor, VisitorQueue};

/// Bridge-owned dialogue dispatch gate.
///
/// Holds the ID of the quest whose `Talk` stage was most
/// recently dispatched, or `None` once the dialogue has
/// actually ended. Single-slot because yarn dialogue is serial
/// — a second `Talk` stage cannot dispatch while this slot is
/// occupied. Cleared by the `NodeCompleted` observer on the
/// frame the matching yarn node finishes; never cleared by
/// unrelated node completions.
#[derive(Resource, Debug, Default)]
pub struct DialogueInFlight(pub Option<Id<Quest>>);

/// Yarn variable name the bridge treats as "the player's choice"
/// when deciding which `Talk` branch to follow. A Yarn node
/// writes this inside the option the player picked:
///
/// ```yarn
/// -> I'll help.
///     <<set $quest_choice = "accept">>
/// ```
const CHOICE_VAR: &str = "$quest_choice";

/// Prefix used to filter which Yarn variables get captured back
/// into the quest flag bag. Everything that matches is copied
/// verbatim — later stages can then branch on them via
/// [`ObjectiveCondition::QuestFlag`](cordon_core::world::narrative::ObjectiveCondition::QuestFlag).
const FLAG_PREFIX: &str = "$quest_";

/// Dispatch `Talk` stages to the dialogue runner. For each
/// active quest parked on a fresh `Talk` stage, either enqueue
/// a [`Visitor`] (when the stage has an NPC — the normal case)
/// or fire [`StartDialogue`] directly (narrator-only). In both
/// cases the quest ID is latched into [`DialogueInFlight`] so
/// the same stage isn't dispatched twice.
///
/// Narrator-only dialogue bypasses the visitor queue entirely
/// and only dispatches when the in-flight slot is empty — a
/// visitor-driven dialogue already running will always
/// complete first.
pub fn enqueue_talk_dialogue(
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    mut queue: ResMut<VisitorQueue>,
    mut in_flight: ResMut<DialogueInFlight>,
    mut start_dialogue: MessageWriter<StartDialogue>,
    mut spawn_npc: MessageWriter<SpawnNpcRequest>,
) {
    let catalog = &data.0;
    for active in &log.active {
        // Slot is occupied by another quest's dialogue — wait.
        if in_flight.0.is_some() {
            return;
        }
        let Some(def) = catalog.quests.get(&active.def_id) else {
            continue;
        };
        let Some(stage) = def.stage(&active.current_stage) else {
            continue;
        };
        let QuestStageKind::Talk(talk) = &stage.kind else {
            continue;
        };
        let stage_npc = &talk.npc;
        let yarn_node = &talk.yarn_node;

        // Visitor-driven path: stage or quest names an NPC
        // template. If it resolves to a real catalog template,
        // fire a SpawnNpcRequest with the yarn node attached —
        // the NPC walks from its faction settlement to the
        // bunker, and the arrival handler enqueues the Visitor.
        // If it does not resolve, fall back to the legacy
        // "synthesize a visitor from quest.giver" path so
        // string-tagged quests keep working.
        if let Some(template) = stage_npc.as_ref().or(def.giver.as_ref()) {
            if catalog.npc_template(template).is_some() {
                // Re-dispatch is safe: if the template is already
                // alive (idling at home from a prior quest), the
                // SpawnNpcRequest handler strips their current
                // squad and starts a fresh travel leg toward the
                // bunker — same entity, new goal. Repeated sends
                // within a single stage are prevented by
                // `in_flight` being set below, checked at the top
                // of the loop.
                spawn_npc.write(SpawnNpcRequest {
                    template: template.clone(),
                    at: None,
                    yarn_node: Some(yarn_node.clone()),
                });
                in_flight.0 = Some(active.def_id.clone());
                info!(
                    "quest `{}` dispatched template `{}` to travel for Talk stage `{}`",
                    active.def_id.as_str(),
                    template.as_str(),
                    active.current_stage.as_str()
                );
                return;
            }
            let faction = def
                .giver_faction
                .clone()
                .unwrap_or_else(|| Id::<Faction>::new("faction_drifters"));
            queue.0.push_back(Visitor {
                display_name: template.as_str().to_string(),
                faction,
                yarn_node: yarn_node.clone(),
            });
            in_flight.0 = Some(active.def_id.clone());
            info!(
                "quest `{}` enqueued legacy visitor `{}` for Talk stage `{}`",
                active.def_id.as_str(),
                template.as_str(),
                active.current_stage.as_str()
            );
            return;
        }

        // Narrator path: no NPC, fire the yarn node directly
        // at the runner. The dialogue UI is gated on the bunker
        // state so the player must be at the desk for the
        // narrator lines to render — same constraint as
        // visitor Talk stages.
        start_dialogue.write(StartDialogue {
            node: yarn_node.clone(),
        });
        in_flight.0 = Some(active.def_id.clone());
        info!(
            "quest `{}` started narrator node `{}` for stage `{}`",
            active.def_id.as_str(),
            yarn_node,
            active.current_stage.as_str()
        );
        return;
    }
}

/// Bevy observer: `NodeCompleted` fires once Yarn has finished
/// running a specific node and carries that node's name.
///
/// Looks up the active quest whose current Talk stage's
/// `yarn_node` matches the just-ended node, drains the runner's
/// Yarn variables into that quest's flag bag, and advances the
/// stage. Matching by node name (instead of a single-slot
/// `DialogueInFlight`) prevents a race where some *other*
/// dialogue ending between the SpawnNpcRequest and the actual
/// Talk arrival drains the slot for the wrong quest.
pub fn on_dialogue_completed(
    event: On<NodeCompleted>,
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    mut in_flight: ResMut<DialogueInFlight>,
    mut dismiss: MessageWriter<DismissTemplateNpc>,
    mut talk_completed: MessageWriter<TalkCompleted>,
    runner_q: Query<&DialogueRunner>,
    template_q: Query<(Entity, &TemplateId)>,
) {
    let node_name = &event.node_name;
    // Find the active quest whose current Talk stage ran this
    // yarn node. If no match, the node was unrelated to any
    // quest — do nothing.
    let quest_id = log.active.iter().find_map(|active| {
        let def = data.0.quests.get(&active.def_id)?;
        let stage = def.stage(&active.current_stage)?;
        match &stage.kind {
            QuestStageKind::Talk(talk) if talk.yarn_node == *node_name => {
                Some(active.def_id.clone())
            }
            _ => None,
        }
    });
    let Some(quest_id) = quest_id else {
        return;
    };
    // Release the dispatch gate — this is the only place a
    // quest's in-flight slot is cleared now.
    if in_flight.0.as_ref() == Some(&quest_id) {
        in_flight.0 = None;
    }

    // Dismiss the template NPC that just finished talking.
    let dismissed_template = log
        .active_instance(&quest_id)
        .and_then(|active| data.0.quests.get(&active.def_id).map(|def| (active, def)))
        .and_then(|(active, def)| def.stage(&active.current_stage))
        .and_then(|stage| match &stage.kind {
            QuestStageKind::Talk(talk) => talk.npc.clone(),
            _ => None,
        });
    if let Some(template_id) = dismissed_template {
        for (entity, tid) in &template_q {
            if tid.0 == template_id {
                dismiss.write(DismissTemplateNpc {
                    entity,
                    template: template_id.clone(),
                });
                info!(
                    "quest `{}`: template `{}` dismissed after dialogue",
                    quest_id.as_str(),
                    template_id.as_str()
                );
                break;
            }
        }
    }

    // Drain Yarn variables into the quest's flag bag.
    let Ok(runner) = runner_q.single() else {
        warn!("DialogueCompleted: no dialogue runner entity found");
        return;
    };
    let variables = runner.variable_storage().variables();

    let mut captured_choice: Option<String> = None;
    if let Some(active) = log.active_instance_mut(&quest_id) {
        for (name, value) in variables {
            if !name.starts_with(FLAG_PREFIX) {
                continue;
            }
            if name == CHOICE_VAR
                && let YarnValue::String(s) = &value
            {
                captured_choice = Some(s.clone());
            }
            active.flags.insert(name, value);
        }
    } else {
        warn!(
            "DialogueCompleted: quest `{}` not in active log",
            quest_id.as_str()
        );
        return;
    }

    // Emit message — the sim-side drive system handles the
    // stage advance.
    talk_completed.write(TalkCompleted {
        quest: quest_id,
        choice: captured_choice,
    });
}
