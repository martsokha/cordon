use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{PlayerDecisions, PlayerIdentity, PlayerStash};

use super::mirror;
use super::registry::YarnCommandRegistry;
use crate::bunker::rack::Carrying;
use crate::bunker::rack::components::RackSlot;
use crate::bunker::resources::{
    CurrentDialogue, CurrentDialogueOwner, DialogueChoice, DialogueOptionView, DialogueOwner,
    OptionsPrompt, PendingOptionsPrompt, StartDialogue, StopDialogue,
};

/// Marker for the single active dialogue runner entity. Internal —
/// other modules go through [`StartDialogue`] / [`DialogueChoice`]
/// instead of touching the runner directly. `pub(super)` so the
/// mirror subsystem (sibling module) can query for it.
#[derive(Component)]
pub(super) struct DialogueRunnerMarker;

pub(super) fn spawn_dialogue_runner(
    mut commands: Commands,
    project: Res<YarnProject>,
    registry: Res<YarnCommandRegistry>,
) {
    let mut runner = project.create_dialogue_runner(&mut commands);
    // Install every plugin-contributed yarn command. The
    // registry is populated at plugin-build time via
    // `AppYarnCommandExt::add_yarn_command`; systems here just
    // drain it onto the fresh runner.
    registry.bind_all(runner.commands_mut());
    commands.spawn((DialogueRunnerMarker, runner));
}

pub(super) fn on_present_line(
    event: On<PresentLine>,
    mut current: ResMut<CurrentDialogue>,
    mut pending_prompt: ResMut<PendingOptionsPrompt>,
    mut choices: MessageWriter<DialogueChoice>,
) {
    let speaker = event.line.character_name().map(|s: &str| s.to_string());
    let text = event.line.text_without_character_name();
    // `#autocontinue` is yarn-author's way of saying "this line is a
    // prompt header for the options that follow — don't make the
    // player click Continue to get past it." We publish the line
    // into `CurrentDialogue::Line` AND stash the text into
    // `PendingOptionsPrompt`; `on_present_options` reads the stash
    // when the following options fire and embeds the prompt into
    // the Options state. That avoids relying on Line→Options frame
    // ordering in the UI sync.
    let autocontinue = event.line.metadata.iter().any(|m| m == "autocontinue");
    if autocontinue {
        pending_prompt.0 = Some(OptionsPrompt {
            speaker: speaker.clone(),
            text: text.clone(),
        });
        choices.write(DialogueChoice::Continue);
    } else {
        // A normal (non-autocontinue) line invalidates any pending
        // prompt — if yarn authored a separate line between an
        // earlier `#autocontinue` and the options, the earlier
        // prompt is no longer the header.
        pending_prompt.0 = None;
    }
    *current = CurrentDialogue::Line {
        speaker,
        text,
        autocontinue,
    };
}

pub(super) fn on_present_options(
    event: On<PresentOptions>,
    mut current: ResMut<CurrentDialogue>,
    mut pending_prompt: ResMut<PendingOptionsPrompt>,
) {
    let lines = event
        .options
        .iter()
        .map(|opt| DialogueOptionView {
            id: opt.id,
            text: opt.line.text_without_character_name(),
            available: opt.is_available,
            // `#hide` metadata marks options that should be
            // skipped entirely when unavailable, rather than
            // greyed. See `DialogueOptionView::hide_when_unavailable`.
            hide_when_unavailable: opt.line.metadata.iter().any(|m| m == "hide"),
        })
        .collect();
    // Consume the pending autocontinue prompt (if any) as the
    // header for this options block. `take()` clears the resource
    // so a subsequent options block without a preceding
    // autocontinue line renders without a stale prompt.
    let prompt = pending_prompt.0.take();
    *current = CurrentDialogue::Options { lines, prompt };
}

/// `DialogueCompleted` observer: set the UI back to idle. This is
/// the single source of truth for "dialog panel should hide now."
pub(super) fn on_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut current: ResMut<CurrentDialogue>,
    mut pending_prompt: ResMut<PendingOptionsPrompt>,
) {
    *current = CurrentDialogue::Idle;
    // A dangling autocontinue prompt from a just-ended dialog must
    // not bleed into the next one.
    pending_prompt.0 = None;
}

/// `DialogueCompleted` observer: latch the owner-tag reset so
/// `reset_dialogue_owner` wipes it next frame. Split from
/// `on_dialogue_completed` because the owner bookkeeping has its
/// own concern (attribution for other subsystems' observers) and
/// shouldn't be tangled with "UI state goes to idle."
///
/// ## The indirection (`OwnerPendingClear` → `reset_dialogue_owner`)
///
/// Bevy observers fire synchronously on their trigger in an
/// undefined order. If we cleared the owner tag here, sibling
/// observers of the same `DialogueCompleted` (`handle_quest_dialogue_end`,
/// `on_broadcast_dialogue_completed`) might run after us and see
/// `None` — losing the "was this mine?" signal they need.
///
/// Solution: latch a "clear next frame" flag here, run
/// [`reset_dialogue_owner`] as a normal (ordered) system on the
/// next frame's `Update`. Every observer on the completion frame
/// still sees the real owner; the clear happens afterwards.
pub(super) fn on_dialogue_completed_latch_owner_clear(
    _event: On<DialogueCompleted>,
    mut owner_pending_clear: ResMut<OwnerPendingClear>,
) {
    owner_pending_clear.0 = true;
}

/// One-shot latch: true while a `DialogueCompleted` fired this
/// frame and the owner tag should be reset next frame. Exists
/// only so [`reset_dialogue_owner`] can run as a normal system
/// (ordered) instead of an observer (unordered), keeping the
/// owner tag readable from every other observer on the completion
/// frame.
#[derive(Resource, Debug, Default)]
pub(super) struct OwnerPendingClear(bool);

pub(super) fn reset_dialogue_owner(
    mut latch: ResMut<OwnerPendingClear>,
    mut owner: ResMut<CurrentDialogueOwner>,
) {
    if !latch.0 {
        return;
    }
    // Unconditionally clear — if a fresh `StartDialogue` was
    // written this frame, [`apply_start_dialogue`] (ordered after
    // this system) overwrites the owner tag with the new value
    // on the same frame, so we never end up with a stale `None`
    // when a dialog is actually starting.
    owner.0 = DialogueOwner::None;
    latch.0 = false;
}

pub(super) fn apply_start_dialogue(
    mut requests: MessageReader<StartDialogue>,
    mut runner_q: Query<&mut DialogueRunner, With<DialogueRunnerMarker>>,
    mut owner: ResMut<CurrentDialogueOwner>,
    carrying: Option<Res<Carrying>>,
    stash: Option<Res<PlayerStash>>,
    identity: Option<Res<PlayerIdentity>>,
    decisions: Option<Res<PlayerDecisions>>,
    game_data: Option<Res<GameDataResource>>,
    rack_slots: Query<&RackSlot>,
) {
    let pending: Vec<_> = requests.read().collect();
    if pending.is_empty() {
        return;
    }
    let Ok(mut runner) = runner_q.single_mut() else {
        // No runner means `spawn_dialogue_runner` never fired —
        // usually because `YarnProject` failed to load. Warn loudly
        // so a silent "dialogue does nothing" bug surfaces in logs.
        for req in &pending {
            warn!(
                "StartDialogue: no DialogueRunner; dropping node `{}`. \
                 Check that the YarnProject loaded.",
                req.node
            );
        }
        return;
    };
    // Push mirrored state *before* starting the node so the
    // node's first `<<if>>` evaluation sees current player
    // inventory and credits, even if nothing changed since the
    // last conversation. Mid-dialogue changes go through
    // `mirror::mirror_on_change`.
    //
    // Player-state resources are `Option` because StartDialogue
    // *could* theoretically fire outside a run (e.g. if some
    // menu-level system sent one). In practice all current
    // senders are `PlayingState::Bunker`-gated, so when these
    // are `None` we fall through to `start_node` on the raw
    // runner — yarn with no mirrored vars will just evaluate
    // every `<<if>>` guard as false, which is the right
    // defensive default.
    if let (Some(carrying), Some(stash), Some(identity), Some(decisions), Some(game_data)) =
        (carrying, stash, identity, decisions, game_data)
    {
        mirror::push_snapshot(
            &mut runner,
            &carrying,
            &stash,
            &identity,
            &decisions,
            &game_data,
            &rack_slots,
        );
    }
    // Last writer wins on both the yarn node and the owner tag.
    // The runner's `start_node` also takes the last call, so the
    // two stay consistent. Multiple writes per frame shouldn't
    // happen in practice — radio gates on `CurrentDialogue::Idle`,
    // quest holds the `DialogueInFlight` slot — but this ordering
    // keeps the state consistent if they ever do.
    for req in pending {
        owner.0 = req.by;
        runner.start_node(&req.node);
    }
}

/// Consume [`StopDialogue`] messages by stopping the runner — but
/// only if it's actually running. Calling `runner.stop()` on an
/// idle runner still queues stop events, which the
/// [`on_dialogue_completed`] observer would consume as "a dialog
/// just ended with no choice" and let the quest's fallback outcome
/// apply. On game start / reset there's no dialog to stop, so the
/// no-op check prevents a spurious Talk-stage fallback.
///
/// When the runner *is* running, `runner.stop()` queues its own
/// completion events and the observer writes
/// `CurrentDialogue::Idle` — so we don't touch `CurrentDialogue`
/// here.
pub(super) fn apply_stop_dialogue(
    mut requests: MessageReader<StopDialogue>,
    mut runner_q: Query<&mut DialogueRunner, With<DialogueRunnerMarker>>,
) {
    if requests.read().next().is_none() {
        return;
    }
    if let Ok(mut runner) = runner_q.single_mut()
        && runner.is_running()
    {
        runner.stop();
    }
}

pub(super) fn apply_player_choice(
    mut choices: MessageReader<DialogueChoice>,
    mut runner_q: Query<&mut DialogueRunner, With<DialogueRunnerMarker>>,
) {
    let pending: Vec<_> = choices.read().copied().collect();
    if pending.is_empty() {
        return;
    }
    let Ok(mut runner) = runner_q.single_mut() else {
        warn!(
            "DialogueChoice: no DialogueRunner; dropping {} choice(s).",
            pending.len()
        );
        return;
    };
    for choice in pending {
        match choice {
            DialogueChoice::Continue => {
                runner.continue_in_next_update();
            }
            DialogueChoice::Option { id } => {
                let _ = runner.select_option(id);
            }
        }
    }
}
