use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;

use crate::bunker::resources::{
    CurrentDialogue, DialogueChoice, DialogueOptionView, StartDialogue, StopDialogue,
};

/// Marker for the single active dialogue runner entity. Internal —
/// other modules go through [`StartDialogue`] / [`DialogueChoice`]
/// instead of touching the runner directly.
#[derive(Component)]
pub(super) struct DialogueRunnerMarker;

pub(super) fn spawn_dialogue_runner(mut commands: Commands, project: Res<YarnProject>) {
    let runner = project.create_dialogue_runner(&mut commands);
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
