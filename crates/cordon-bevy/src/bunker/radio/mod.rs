//! Bunker radio: toggleable spatial static loop with broadcast
//! chatter on [`RadioBroadcast`] messages from the sim layer.

mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::load_sfx);
        app.add_systems(
            Update,
            (systems::spawn_radio, systems::play_broadcast).run_if(in_state(PlayingState::Bunker)),
        );
    }
}
