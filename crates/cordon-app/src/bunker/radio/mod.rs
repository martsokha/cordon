//! Bunker radio: toggleable spatial static loop, broadcast chatter
//! stings, and player-initiated broadcast dialogue playback.
//!
//! Broadcasts arrive from the sim layer and queue up until the
//! player turns the radio on. Each broadcast carries a yarn node
//! that runs as dialogue when played; intel grants on dialogue
//! completion, not on queue delivery.

mod queue;
mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<queue::RadioQueue>();
        app.init_resource::<queue::ActiveBroadcast>();
        app.add_systems(Startup, systems::load_sfx);
        app.add_systems(
            Update,
            (
                systems::spawn_radio,
                systems::play_broadcast,
                queue::on_radio_broadcast,
                queue::try_start_next_broadcast,
                queue::prune_missable_on_day_roll,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
        // Listens to DialogueCompleted; grants broadcast intel when
        // a queued broadcast's yarn node ends.
        app.add_observer(queue::on_broadcast_dialogue_completed);
    }
}
