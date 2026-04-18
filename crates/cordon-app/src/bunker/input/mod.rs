//! Bunker input: FPS controls, cursor management, and input-
//! triggered feedback (footstep audio).

pub(crate) mod controller;
mod footsteps;
mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(controller::ControllerPlugin);
        footsteps::plugin(app);
        app.add_systems(OnEnter(PlayingState::Bunker), systems::grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), systems::hide_interact_prompt);
    }
}
