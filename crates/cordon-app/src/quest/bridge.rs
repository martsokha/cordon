//! The concrete glue between quest state and the dialogue runner.
//!
//! Three entry points:
//!
//! - [`enqueue_talk_dialogue`] runs every frame in the bunker
//!   state. For each active quest parked on a `Talk` stage
//!   that hasn't been dispatched yet, fires either
//!   [`SpawnNpcRequest`] (template NPC walks to the bunker) or
//!   [`StartDialogue`] (narrator-only). The bridge-owned
//!   [`DialogueInFlight`] slot is set at dispatch so the same
//!   stage isn't double-dispatched while the visitor is
//!   still travelling.
//! - [`quest_advance_command`] is the yarn-callable
//!   `<<quest_advance "branch">>` command. The yarn author
//!   calls it from within the Talk stage's yarn node at the
//!   moment the player's choice should commit the quest (e.g.
//!   inside an option body). The command identifies the quest
//!   by looking up whichever active quest has a Talk stage
//!   whose `yarn_node` matches the currently-executing node —
//!   no quest id in the yarn source.
//! - [`handle_quest_dialogue_end`] clears
//!   [`DialogueInFlight`] when yarnspinner fires
//!   `DialogueCompleted` (whole conversation ended). That's
//!   the only thing that frees the dispatch gate now.
//!
//! Dialogue is strictly serial: at any moment there is at most
//! one quest's Talk stage dispatched. The visitor queue keeps
//! visitor-driven dialogue serial; narrator-only stages only
//! dispatch when the slot is already empty.

use bevy::ecs::system::In;
use bevy::prelude::*;
use bevy_yarnspinner::events::DialogueCompleted;
use bevy_yarnspinner::prelude::DialogueRunner;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Quest, QuestStageKind};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::QuestLog;
use cordon_sim::quest::messages::{SpawnNpcRequest, TalkCompleted};

use crate::bunker::resources::{CurrentDialogueOwner, DialogueOwner, StartDialogue};

/// Bridge-owned dialogue dispatch gate.
///
/// Holds the ID of the quest whose `Talk` stage was most
/// recently dispatched, or `None` once the conversation has
/// ended. Single-slot because yarn dialogue is serial — a
/// second `Talk` stage cannot dispatch while this slot is
/// occupied. Cleared only by [`handle_quest_dialogue_end`]
/// on `DialogueCompleted`.
#[derive(Resource, Debug, Default)]
pub struct DialogueInFlight(pub Option<Id<Quest>>);

/// Prefix used to filter which Yarn variables get captured back
/// into the quest flag bag. Everything that matches is copied
/// verbatim — later stages can then branch on them via
/// [`ObjectiveCondition::QuestFlag`](cordon_core::world::narrative::ObjectiveCondition::QuestFlag).
const FLAG_PREFIX: &str = "$quest_";

/// Dispatch `Talk` stages to the dialogue runner. For each
/// active quest parked on a fresh `Talk` stage, either fire a
/// [`SpawnNpcRequest`] so the NPC travels to the bunker (the
/// normal case) or fire [`StartDialogue`] directly for narrator-
/// only stages. In both cases the quest ID is latched into
/// [`DialogueInFlight`] so the same stage isn't dispatched twice.
///
/// Narrator-only dialogue only dispatches when the in-flight
/// slot is empty — a visitor-driven dialogue already running
/// will always complete first.
pub(super) fn enqueue_talk_dialogue(
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
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
        // template. Fire a SpawnNpcRequest with the yarn node
        // attached — the NPC walks from its faction settlement to
        // the bunker, and the arrival handler enqueues the
        // Visitor. An unknown template id is an authoring error
        // (quest json references a non-existent NPC); log it and
        // fall through to the narrator path so the stage still
        // advances instead of silently hanging.
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
                    delivery_items: Vec::new(),
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
            error!(
                "quest `{}` Talk stage `{}` references unknown NPC template `{}`; \
                 fix the quest json or register the template",
                active.def_id.as_str(),
                active.current_stage.as_str(),
                template.as_str()
            );
        }

        // Narrator path: no NPC, fire the yarn node directly
        // at the runner. The dialogue UI is gated on the bunker
        // state so the player must be at the desk for the
        // narrator lines to render — same constraint as
        // visitor Talk stages.
        start_dialogue.write(StartDialogue {
            node: yarn_node.clone(),
            by: DialogueOwner::Quest,
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

/// Yarn-callable command: `<<quest_advance "branch">>`.
///
/// The yarn author calls this from within a Talk-stage yarn
/// node at the moment the player's choice should commit the
/// quest. The command identifies the quest by looking up which
/// active quest has a Talk stage whose `yarn_node` matches the
/// currently-executing node — no quest id needs to appear in
/// the yarn source. If no active quest's Talk stage claims the
/// current node, the command warns and no-ops (yarn may be
/// running for a non-quest reason, e.g. casual trade).
///
/// **Ordering relative to `<<jump>>`**: call `quest_advance`
/// *before* any `<<jump>>` in the same option body. A jump
/// switches the runner's `current_node()` immediately, so a
/// post-jump `quest_advance` would resolve against the
/// destination node (probably not a Talk stage) and no-op.
///
/// What it does:
///
/// - Drains `$quest_*` yarn variables (other than the choice
///   itself) into the quest's flag bag so later stages can
///   branch on them.
/// - Emits [`TalkCompleted`] with the branch choice; the sim
///   layer advances the stage in response.
///
/// Notably does NOT dismiss the template NPC. The sim NPC
/// stays in the bunker until the bunker-side conversation
/// actually ends — the visitor lifecycle fires
/// [`DismissTemplateNpc`] when the sprite despawns. That keeps
/// the sprite and the sim entity visibly paired: they appear
/// together at admit time, and leave together at end-of-trade.
///
/// Does NOT clear [`DialogueInFlight`] — that happens when the
/// whole conversation ends (see [`handle_quest_dialogue_end`]).
pub(super) fn quest_advance_command(
    In(choice): In<String>,
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    mut talk_completed: MessageWriter<TalkCompleted>,
    runner_q: Query<&DialogueRunner>,
) {
    let Ok(runner) = runner_q.single() else {
        warn!("quest_advance `{choice}`: no DialogueRunner entity");
        return;
    };
    let Some(current_node) = runner.current_node() else {
        warn!("quest_advance `{choice}`: runner has no current node");
        return;
    };

    // Find the active quest whose current Talk stage targets
    // the currently-executing yarn node. A non-match is OK —
    // yarn may be running this node for a non-quest reason
    // (casual trade, tutorial, etc.).
    let quest_id = log.active.iter().find_map(|active| {
        let def = data.0.quests.get(&active.def_id)?;
        let stage = def.stage(&active.current_stage)?;
        match &stage.kind {
            QuestStageKind::Talk(talk) if talk.yarn_node == current_node => {
                Some(active.def_id.clone())
            }
            _ => None,
        }
    });
    let Some(quest_id) = quest_id else {
        warn!(
            "quest_advance `{choice}`: no active quest claims yarn node `{current_node}`; \
             command ignored"
        );
        return;
    };

    // Drain `$quest_*` yarn variables into the quest's flag
    // bag (excluding the choice itself, which travels as an
    // explicit `TalkCompleted` field).
    let variables = runner.variable_storage().variables();
    if let Some(active) = log.active_instance_mut(&quest_id) {
        for (name, value) in variables {
            if !name.starts_with(FLAG_PREFIX) {
                continue;
            }
            active.flags.insert(name, value);
        }
    } else {
        warn!(
            "quest_advance `{choice}`: quest `{}` not in active log",
            quest_id.as_str()
        );
        return;
    }

    talk_completed.write(TalkCompleted {
        quest: quest_id,
        choice: Some(choice),
    });
}

/// Observer on `DialogueCompleted`: clear the dispatch gate so
/// the next Talk stage can dispatch, and auto-complete the talk
/// stage if the yarn node ended without calling `<<quest_advance>>`.
///
/// Auto-completion fires a choice-less [`TalkCompleted`] so the
/// quest advances to its `fallback`. Yarn nodes with no branches
/// (flavor-only talk stages like tenant whispers) progress just by
/// running to end-of-node — authors don't have to sprinkle
/// `<<quest_advance>>` everywhere. Nodes that *did* call
/// `quest_advance` already emitted their `TalkCompleted`; by the
/// time we run, the quest's stage has moved off `Talk`, so the
/// drive handler's talk-stage guard silently ignores the second
/// emission.
///
/// Only acts on dialogs tagged [`DialogueOwner::Quest`]. Radio
/// broadcasts and visitor conversations use their own owner tags
/// and are silently skipped here — the quest subsystem is fully
/// insulated from unrelated dialog completions.
pub(super) fn handle_quest_dialogue_end(
    _event: On<DialogueCompleted>,
    mut in_flight: ResMut<DialogueInFlight>,
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    owner: Res<CurrentDialogueOwner>,
    mut talk_completed: MessageWriter<TalkCompleted>,
) {
    if !matches!(owner.0, DialogueOwner::Quest) {
        return;
    }
    let quest_id = in_flight.0.take();
    let Some(quest_id) = quest_id else {
        return;
    };
    // If the dispatched talk stage is still the quest's current
    // stage, no `<<quest_advance>>` fired — emit a default
    // TalkCompleted so the quest progresses to `fallback`.
    let still_on_talk = log
        .active_instance(&quest_id)
        .and_then(|active| {
            let def = data.0.quests.get(&active.def_id)?;
            let stage = def.stage(&active.current_stage)?;
            matches!(stage.kind, QuestStageKind::Talk(_)).then_some(())
        })
        .is_some();
    if still_on_talk {
        talk_completed.write(TalkCompleted {
            quest: quest_id,
            choice: None,
        });
    }
}
