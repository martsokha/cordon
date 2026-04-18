use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;

use crate::bunker::resources::{
    CurrentDialogue, DialogueChoice, DialogueOptionView, StartDialogue,
};

/// Marker for the single active dialogue runner entity. Internal —
/// other modules go through [`StartDialogue`] / [`DialogueChoice`]
/// instead of touching the runner directly.
#[derive(Component)]
pub(super) struct ActiveRunner;

pub(super) fn spawn_dialogue_runner(mut commands: Commands, project: Res<YarnProject>) {
    let runner = project.create_dialogue_runner(&mut commands);
    commands.spawn((ActiveRunner, runner));
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
    mut runner_q: Query<&mut DialogueRunner, With<ActiveRunner>>,
) {
    let Ok(mut runner) = runner_q.single_mut() else {
        return;
    };
    for req in requests.read() {
        runner.start_node(&req.node);
    }
}

pub(super) fn apply_player_choice(
    mut choices: MessageReader<DialogueChoice>,
    mut runner_q: Query<&mut DialogueRunner, With<ActiveRunner>>,
) {
    let Ok(mut runner) = runner_q.single_mut() else {
        return;
    };
    for choice in choices.read() {
        match choice {
            DialogueChoice::Continue => {
                runner.continue_in_next_update();
            }
            DialogueChoice::Option { id } => {
                let _ = runner.select_option(*id);
            }
        }
    }
}
