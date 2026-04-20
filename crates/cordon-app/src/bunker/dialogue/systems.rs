use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{PlayerIdentity, PlayerStash};

use super::mirror;
use super::registry::YarnCommandRegistry;
use crate::bunker::rack::Carrying;
use crate::bunker::rack::components::RackSlot;
use crate::bunker::resources::{
    CurrentDialogue, DialogueChoice, DialogueOptionView, StartDialogue, StopDialogue,
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

pub(super) fn on_present_line(event: On<PresentLine>, mut current: ResMut<CurrentDialogue>) {
    let speaker = event.line.character_name().map(|s: &str| s.to_string());
    let text = event.line.text_without_character_name();
    *current = CurrentDialogue::Line { speaker, text };
}

pub(super) fn on_present_options(event: On<PresentOptions>, mut current: ResMut<CurrentDialogue>) {
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
    *current = CurrentDialogue::Options { lines };
}

pub(super) fn on_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut current: ResMut<CurrentDialogue>,
) {
    *current = CurrentDialogue::Idle;
}

pub(super) fn apply_start_dialogue(
    mut requests: MessageReader<StartDialogue>,
    mut runner_q: Query<&mut DialogueRunner, With<DialogueRunnerMarker>>,
    carrying: Option<Res<Carrying>>,
    stash: Option<Res<PlayerStash>>,
    identity: Option<Res<PlayerIdentity>>,
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
    if let (Some(carrying), Some(stash), Some(identity), Some(game_data)) =
        (carrying, stash, identity, game_data)
    {
        mirror::push_snapshot(
            &mut runner,
            &carrying,
            &stash,
            &identity,
            &game_data,
            &rack_slots,
        );
    }
    for req in pending {
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
