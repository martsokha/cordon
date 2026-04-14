//! Bunker input: FPS controls and cursor management.

pub mod controller;
mod systems;

use bevy::prelude::*;

use self::controller::ControllerPlugin;
use self::systems::{grab_cursor, hide_interact_prompt};
use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ControllerPlugin);
        app.add_systems(OnEnter(PlayingState::Bunker), grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), hide_interact_prompt);
    }
}
