//! Bunker input: FPS controls and cursor management.

pub mod controller;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::components::InteractPrompt;
use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(controller::ControllerPlugin);
        app.add_systems(OnEnter(PlayingState::Bunker), grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), hide_interact_prompt);
    }
}

fn grab_cursor(mut cursor_q: Query<&mut CursorOptions>) {
    for mut cursor in &mut cursor_q {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn hide_interact_prompt(mut prompt_q: Query<&mut Visibility, With<InteractPrompt>>) {
    for mut vis in &mut prompt_q {
        *vis = Visibility::Hidden;
    }
}
