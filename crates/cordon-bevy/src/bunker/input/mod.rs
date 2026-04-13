//! Bunker input: FPS controls and cursor management.

pub mod controller;
mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(controller::ControllerPlugin);
        app.add_systems(OnEnter(PlayingState::Bunker), systems::grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), systems::hide_interact_prompt);
    }
}
