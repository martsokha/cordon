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
        app.init_resource::<super::resources::CurrentDialogueOwner>();
        app.init_resource::<super::resources::PendingOptionsPrompt>();
        app.init_resource::<systems::OwnerPendingClear>();
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
                // Deterministic order for the dialog lifecycle:
                //
                // 1. `reset_dialogue_owner` clears last-frame's
                //    owner tag (if a DialogueCompleted latched it).
                // 2. `apply_stop_dialogue` processes any pending
                //    `StopDialogue` — if both a start and stop are
                //    queued the same frame, the stop runs first so
                //    the runner is cleanly idle before the new
                //    start begins.
                // 3. `apply_start_dialogue` kicks off any new
                //    dialog requested this frame, setting the
                //    fresh owner tag.
                // 4. `apply_player_choice` processes button
                //    clicks / key shortcuts.
                // 5. `mirror_on_change` pushes player state into
                //    yarn variables.
                systems::reset_dialogue_owner,
                systems::apply_stop_dialogue.after(systems::reset_dialogue_owner),
                systems::apply_start_dialogue.after(systems::apply_stop_dialogue),
                systems::apply_player_choice.after(systems::apply_start_dialogue),
                // Cheap; early-outs unless a watched resource changed.
                mirror::mirror_on_change,
            ),
        );
        app.add_observer(systems::on_present_line);
        app.add_observer(systems::on_present_options);
        app.add_observer(systems::on_dialogue_completed);
        app.add_observer(systems::on_dialogue_completed_latch_owner_clear);
    }
}
