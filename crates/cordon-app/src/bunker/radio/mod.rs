//! Bunker radio: player-initiated listening mode with a queue of
//! pending broadcasts, static-filled gaps between them, and an
//! idle placeholder when nothing's pending.
//!
//! Interacting with the radio prop enters a focused conversation
//! state (movement + interaction locked, static loop audible)
//! where queued broadcasts play one after another through the
//! dialogue UI. Intel grants on each broadcast's dialog
//! completing. ESC or the `<<close_radio>>` yarn command exits.

mod queue;
mod systems;

use bevy::prelude::*;

pub use self::queue::{ExitListening, ListeningToRadio, reset_listening_state};

use crate::PlayingState;

pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<queue::RadioQueue>();
        app.init_resource::<queue::ActiveBroadcast>();
        app.init_resource::<queue::ListeningToRadio>();
        app.add_message::<queue::EnterListening>();
        app.add_message::<queue::ExitListening>();
        app.add_systems(Startup, systems::load_sfx);
        app.add_systems(
            Update,
            (
                systems::spawn_radio,
                systems::sync_radio_interactable,
                queue::on_radio_broadcast,
                queue::handle_enter_listening,
                queue::handle_exit_listening,
                queue::tick_static_gap,
                queue::handle_esc_exit,
                queue::prune_missable_on_day_roll,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
        // Listens to DialogueCompleted; grants broadcast intel and
        // schedules whatever comes next (gap timer or idle yarn).
        app.add_observer(queue::on_broadcast_dialogue_completed);
    }
}
