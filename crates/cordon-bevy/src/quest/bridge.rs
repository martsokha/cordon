//! The concrete glue between quest state and the dialogue runner.
//!
//! Two entry points:
//!
//! - [`enqueue_talk_dialogue`] runs every frame in the bunker
//!   state. For each active quest parked on a `Talk` stage
//!   that hasn't been dispatched yet, it either (a) pushes a
//!   [`Visitor`] onto [`VisitorQueue`] when the stage has an
//!   NPC, or (b) fires [`StartDialogue`] directly for
//!   narrator-only stages. Idempotency is handled by a
//!   bridge-owned [`DialogueInFlight`] slot.
//! - [`on_dialogue_completed`] is a Bevy observer on
//!   `DialogueCompleted`. It reads the in-flight quest ID,
//!   copies any `$quest_*` Yarn variables into the quest's
//!   flag bag, calls [`engine::advance_after_talk`] with
//!   whatever `$quest_choice` the Yarn node wrote, and
//!   clears the in-flight slot so the next `Talk` stage is
//!   free to dispatch.
//!
//! Dialogue is strictly serial: at any moment there is at
//! most one quest waiting on a `DialogueCompleted` — either
//! its visitor is inside, or its narrator node is playing —
//! so [`DialogueInFlight`] is a single-slot resource, not a
//! set. The visitor queue keeps visitor-driven dialogue
//! serial; narrator-only stages only dispatch when the slot
//! is already empty.

use bevy::prelude::*;
use bevy_yarnspinner::events::DialogueCompleted;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Quest, QuestStageKind};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::entity::npc::TemplateId;
use cordon_sim::plugin::prelude::{EventLog, GameClock, QuestLog};
use cordon_sim::quest::consequence::{DismissTemplateNpc, SpawnNpcRequest};
use cordon_sim::quest::engine::advance_after_talk;
use cordon_sim::quest::registry::TemplateRegistry;
use cordon_sim::resources::{
    PlayerIdentity, PlayerIntel, PlayerStandings, PlayerStash, PlayerUpgrades,
};

use crate::bunker::resources::StartDialogue;
use crate::bunker::{Visitor, VisitorQueue};

/// Bridge-owned dialogue-in-flight slot.
///
/// Holds the ID of the quest whose `Talk` stage is currently
/// running through the dialogue runner, or `None` when no
/// quest dialogue is active. Single-slot because yarn
/// dialogue is serial — a second `Talk` stage cannot dispatch
/// while this slot is occupied. Cleared by the
/// `DialogueCompleted` observer.
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
#[allow(clippy::too_many_arguments)]
pub fn enqueue_talk_dialogue(
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    registry: Res<TemplateRegistry>,
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
                // Skip re-dispatching while the named template is
                // still in transit (or inside the bunker) from a
                // prior firing of this stage.
                if registry.is_alive(template) {
                    return;
                }
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

/// Bevy observer: `DialogueCompleted` fires once Yarn has
/// finished running the node the player was talking through.
///
/// Reads the in-flight quest ID from [`DialogueInFlight`] —
/// set by [`enqueue_talk_dialogue`] when the stage was
/// dispatched — then drains the runner's Yarn variables into
/// the quest's flag bag and advances the stage. Works
/// identically for visitor-driven and narrator-only dialogue
/// because the slot is the sole source of truth for "which
/// quest is waiting on this event."
#[allow(clippy::too_many_arguments)]
pub fn on_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut log: ResMut<QuestLog>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    identity: Res<PlayerIdentity>,
    standings: Res<PlayerStandings>,
    upgrades: Res<PlayerUpgrades>,
    stash: Res<PlayerStash>,
    intel: Res<PlayerIntel>,
    events: Res<EventLog>,
    registry: Res<TemplateRegistry>,
    mut in_flight: ResMut<DialogueInFlight>,
    mut dismiss: MessageWriter<DismissTemplateNpc>,
    runner_q: Query<&DialogueRunner>,
    template_q: Query<(Entity, &TemplateId)>,
) {
    let Some(quest_id) = in_flight.0.take() else {
        // No quest was waiting on this dialogue — ambient /
        // non-quest dialogue ended. Nothing to do.
        return;
    };

    // Find the template NPC that just finished talking (if any).
    // The quest's current stage hasn't advanced yet, so we look
    // up the Talk stage's `npc` to know which template to retire.
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
                    "quest `{}`: template `{}` dismissed after dialogue, returning home",
                    quest_id.as_str(),
                    template_id.as_str()
                );
                break;
            }
        }
    }

    let Ok(runner) = runner_q.single() else {
        warn!("DialogueCompleted: no dialogue runner entity found");
        return;
    };
    let variables = runner.variable_storage().variables();

    // Copy every $quest_* variable into the active quest's
    // flag bag, overwriting any previous value with the same
    // key. This includes `$quest_choice` itself so later
    // conditions can read it via `QuestFlag` too.
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
            "DialogueCompleted: visitor tagged quest `{}` not in active log",
            quest_id.as_str()
        );
        return;
    }

    advance_after_talk(
        &mut log,
        &data.0,
        &identity,
        &standings,
        &upgrades,
        &stash,
        &intel,
        &events.0,
        &registry,
        &quest_id,
        captured_choice.as_deref(),
        clock.0,
    );
}
