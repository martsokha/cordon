//! Bunker ambient audio: always-on room tone that grounds the
//! silence between gameplay sounds. Starts when the bunker
//! spawns, loops indefinitely.

mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct AmbientPlugin;

impl Plugin for AmbientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            systems::start_ambient.run_if(not(resource_exists::<systems::AmbientSpawned>)),
        );
    }
}
