//! Bunker input: FPS controls, cursor grab, interaction.

pub mod controller;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::{FpsCamera, InteractPrompt, LaptopObject};
use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(controller::ControllerPlugin);
        app.add_systems(OnEnter(PlayingState::Bunker), grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), hide_interact_prompt);
        app.add_systems(
            Update,
            (update_interact_prompt, interact).run_if(in_state(PlayingState::Bunker)),
        );
    }
}

const INTERACT_DIST: f32 = 3.5;

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

fn update_interact_prompt(
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    mut prompt_q: Query<(&mut Text, &mut Visibility), With<InteractPrompt>>,
) {
    let near = is_near_laptop(&camera_q, &laptop_q);
    for (mut text, mut vis) in &mut prompt_q {
        if near {
            text.0 = "[E] Use Laptop".into();
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    if is_near_laptop(&camera_q, &laptop_q) {
        *next_state = NextState::Pending(PlayingState::Laptop);
    }
}

fn is_near_laptop(
    camera_q: &Query<&Transform, With<FpsCamera>>,
    laptop_q: &Query<&Transform, With<LaptopObject>>,
) -> bool {
    let Ok(cam) = camera_q.single() else {
        return false;
    };
    let Ok(laptop) = laptop_q.single() else {
        return false;
    };
    let dist = cam.translation.distance(laptop.translation);
    if dist > INTERACT_DIST {
        return false;
    }

    // Check if looking toward the laptop (dot product > 0.5 ≈ within ~60° cone)
    let to_laptop = (laptop.translation - cam.translation).normalize_or_zero();
    let forward = cam.forward().as_vec3();
    forward.dot(to_laptop) > 0.5
}
