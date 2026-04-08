//! Bunker input: FPS controls, cursor grab, interaction.

pub mod controller;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_yarnspinner::prelude::DialogueRunner;

use super::dialogue::ActiveRunner;
use super::visitor::{admit_visitor, VisitorState};
use super::{CameraMode, DoorButton, FpsCamera, InteractPrompt, LaptopObject};
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
    button_q: Query<&Transform, With<DoorButton>>,
    visitor_state: Res<VisitorState>,
    mut prompt_q: Query<(&mut Text, &mut Visibility), With<InteractPrompt>>,
) {
    // Door button takes priority when a visitor is knocking — that's
    // the timely interaction. Both checks are pure proximity, no
    // facing-direction gate.
    let knocking = matches!(*visitor_state, VisitorState::Knocking { .. });
    let near_button = knocking && is_near(&camera_q, &button_q);
    let near_laptop = is_near(&camera_q, &laptop_q);

    for (mut text, mut vis) in &mut prompt_q {
        if near_button {
            text.0 = "[E] Open Door".into();
            *vis = Visibility::Visible;
        } else if near_laptop {
            text.0 = "[E] Use Laptop".into();
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    button_q: Query<&Transform, With<DoorButton>>,
    visitor_state: ResMut<VisitorState>,
    camera_mode: ResMut<CameraMode>,
    runner_q: Query<&mut DialogueRunner, With<ActiveRunner>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    commands: Commands,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let knocking = matches!(*visitor_state, VisitorState::Knocking { .. });
    let in_dialogue = matches!(*visitor_state, VisitorState::Inside { .. });

    // Knocking visitor takes priority over the laptop — admit them.
    if knocking && is_near(&camera_q, &button_q) {
        admit_visitor(
            commands,
            visitor_state,
            camera_mode,
            camera_q,
            runner_q,
            meshes,
            materials,
        );
        return;
    }
    // Block laptop entry while a visitor is in the bunker — the
    // dialogue runs in the bunker view, the laptop is not available
    // until the conversation ends.
    if in_dialogue {
        return;
    }
    if is_near(&camera_q, &laptop_q) {
        *next_state = NextState::Pending(PlayingState::Laptop);
    }
}

/// Pure proximity check. Used by both the laptop and the door
/// button — neither requires the player to be looking *at* the
/// target, just standing near the desk.
fn is_near<M: Component>(
    camera_q: &Query<&Transform, With<FpsCamera>>,
    target_q: &Query<&Transform, With<M>>,
) -> bool {
    let Ok(cam) = camera_q.single() else {
        return false;
    };
    let Ok(target) = target_q.single() else {
        return false;
    };
    cam.translation.distance(target.translation) <= INTERACT_DIST
}
