//! Visitor dialogue runtime.
//!
//! Wraps `bevy_yarnspinner` and exposes two thin layers:
//!
//! - The [`CurrentDialogue`] resource mirrors what the underlying
//!   `DialogueRunner` is currently presenting (line, options, idle).
//!   The UI module ([`ui`]) reads this and renders the visual-novel
//!   text box.
//! - Two messages bridge the runner without exposing yarnspinner
//!   types to other modules:
//!     - [`StartDialogue`] — sent by the visitor module to begin a
//!       conversation at a named yarn node.
//!     - [`DialogueChoice`] — sent by the UI when the player picks
//!       a continue/option button.
//!
//! The Yarn project is loaded once at startup from `assets/dialogue/`
//! and a single `DialogueRunner` entity is spawned as soon as
//! [`YarnProject`] becomes available.
//!
//! `bevy_yarnspinner` 0.8 publishes its events as `EntityEvent`s
//! triggered via `commands.trigger(...)`, so we observe them via
//! Bevy's `On<E>` observer parameter rather than `MessageReader`.

mod commands;
mod mirror;
mod registry;
mod systems;
mod ui;

use bevy::prelude::*;
use bevy_yarnspinner::prelude::*;

pub use self::registry::{AppYarnCommandExt, YarnCommandRegistry};
use super::resources::{CurrentDialogue, DialogueChoice, StartDialogue, StopDialogue};

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((YarnSpinnerPlugin::new(), ui::DialogueUiPlugin));
        app.insert_resource(CurrentDialogue::default());
        app.init_resource::<YarnCommandRegistry>();
        app.add_message::<StartDialogue>();
        app.add_message::<StopDialogue>();
        app.add_message::<DialogueChoice>();
        // Register the built-in trade commands. Other plugins
        // (e.g. QuestBridgePlugin) contribute their own commands
        // through the same `add_yarn_command` extension.
        commands::register(app);
        app.add_systems(
            Update,
            systems::spawn_dialogue_runner.run_if(resource_added::<YarnProject>),
        );
        app.add_systems(
            Update,
            (
                systems::apply_start_dialogue,
                systems::apply_stop_dialogue,
                systems::apply_player_choice,
                // Runs every frame but is cheap: the body early-
                // outs unless a watched resource changed.
                mirror::mirror_on_change,
            ),
        );
        app.add_observer(systems::on_present_line);
        app.add_observer(systems::on_present_options);
        app.add_observer(systems::on_dialogue_completed);
    }
}
