//! Visitor dialogue runtime.
//!
//! Wraps `bevy_yarnspinner` and exposes a [`CurrentDialogue`] resource
//! the trade-tab UI reads to render the active line + choices. The
//! UI in turn writes a [`DialogueChoice`] message when the player
//! picks an option, which this module forwards to the underlying
//! `DialogueRunner`.
//!
//! The Yarn project is loaded once at startup from `assets/dialogue/`
//! and a single `DialogueRunner` entity is spawned as soon as
//! [`YarnProject`] becomes available.
//!
//! `bevy_yarnspinner` 0.8 publishes its events as `EntityEvent`s
//! triggered via `commands.trigger(...)`, so we observe them via
//! Bevy's `On<E>` observer parameter rather than `MessageReader`.

mod ui;

use bevy::prelude::*;
use bevy_yarnspinner::events::{DialogueCompleted, PresentLine, PresentOptions};
use bevy_yarnspinner::prelude::*;

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((YarnSpinnerPlugin::new(), ui::DialogueUiPlugin));
        app.insert_resource(CurrentDialogue::default());
        app.add_message::<DialogueChoice>();
        app.add_systems(
            Update,
            spawn_dialogue_runner.run_if(resource_added::<YarnProject>),
        );
        app.add_systems(Update, apply_player_choice);
        app.add_observer(on_present_line);
        app.add_observer(on_present_options);
        app.add_observer(on_dialogue_completed);
    }
}

/// What the dialogue UI should currently show. Mirrored from the
/// underlying `DialogueRunner` events so the UI doesn't have to know
/// about Yarn types directly.
#[derive(Resource, Default, Debug, Clone)]
pub enum CurrentDialogue {
    /// No dialogue is active.
    #[default]
    Idle,
    /// A line is being shown. The UI should render it and present a
    /// "Continue" affordance that emits a [`DialogueChoice::Continue`].
    Line { speaker: Option<String>, text: String },
    /// A set of options is presented. The UI should render the lines
    /// as buttons; selecting one emits [`DialogueChoice::Option`].
    Options { lines: Vec<DialogueOptionView> },
}

/// Player-facing view of a single dialogue option.
#[derive(Debug, Clone)]
pub struct DialogueOptionView {
    pub id: OptionId,
    pub text: String,
    pub available: bool,
}

/// Player-side message: the UI emits one of these when the player
/// either continues past a line or picks an option.
#[derive(Message, Debug, Clone, Copy)]
pub enum DialogueChoice {
    Continue,
    Option { id: OptionId },
}

/// Component used to find the single active dialogue runner entity.
#[derive(Component)]
pub struct ActiveRunner;

fn spawn_dialogue_runner(mut commands: Commands, project: Res<YarnProject>) {
    let runner = project.create_dialogue_runner(&mut commands);
    commands.spawn((ActiveRunner, runner));
}

fn on_present_line(event: On<PresentLine>, mut current: ResMut<CurrentDialogue>) {
    let speaker = event.line.character_name().map(|s: &str| s.to_string());
    let text = event.line.text_without_character_name();
    *current = CurrentDialogue::Line { speaker, text };
}

fn on_present_options(event: On<PresentOptions>, mut current: ResMut<CurrentDialogue>) {
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

fn on_dialogue_completed(_event: On<DialogueCompleted>, mut current: ResMut<CurrentDialogue>) {
    *current = CurrentDialogue::Idle;
}

fn apply_player_choice(
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
