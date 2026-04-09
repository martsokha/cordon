//! Bunker input: FPS controls, cursor grab, interaction prompts.
//!
//! There are three E-interactables on the bunker side: the door
//! button (only when a visitor is knocking), the laptop, and the
//! CCTV monitor in the corner. They all sit close enough to each
//! other that pure proximity isn't enough to disambiguate, so the
//! input system uses a *facing tiebreaker*: the candidate with
//! the highest forward-dot-product wins.

pub mod controller;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::visitor::{AdmitVisitor, VisitorState};
use super::{CameraMode, CctvMonitor, DoorButton, FpsCamera, InteractPrompt, LaptopObject};
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

/// Which kind of E-interactable the player is currently aimed at.
/// Returned by [`pick_interaction`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interaction {
    Door,
    Laptop,
    Cctv,
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

fn update_interact_prompt(
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    button_q: Query<&Transform, With<DoorButton>>,
    cctv_q: Query<&Transform, With<CctvMonitor>>,
    visitor_state: Res<VisitorState>,
    camera_mode: Res<CameraMode>,
    mut prompt_q: Query<(&mut Text, &mut Visibility), With<InteractPrompt>>,
) {
    // While in CCTV fullscreen, prompt becomes a single "Exit" hint.
    if matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        for (mut text, mut vis) in &mut prompt_q {
            text.0 = "[E] Exit Camera".into();
            *vis = Visibility::Visible;
        }
        return;
    }

    let pick = pick_interaction(&camera_q, &laptop_q, &button_q, &cctv_q, &visitor_state);

    for (mut text, mut vis) in &mut prompt_q {
        match pick {
            Some(Interaction::Door) => {
                text.0 = "[E] Open Door".into();
                *vis = Visibility::Visible;
            }
            Some(Interaction::Laptop) => {
                text.0 = "[E] Use Laptop".into();
                *vis = Visibility::Visible;
            }
            Some(Interaction::Cctv) => {
                text.0 = "[E] View Camera".into();
                *vis = Visibility::Visible;
            }
            None => {
                *vis = Visibility::Hidden;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    button_q: Query<&Transform, With<DoorButton>>,
    cctv_q: Query<&Transform, With<CctvMonitor>>,
    visitor_state: Res<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    mut admit: MessageWriter<AdmitVisitor>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    let pressed_e = keys.just_pressed(KeyCode::KeyE);
    let pressed_esc = keys.just_pressed(KeyCode::Escape);

    // Exit CCTV fullscreen on E or Esc.
    if let CameraMode::AtCctv { saved_transform } = *camera_mode {
        if pressed_e || pressed_esc {
            *camera_mode = CameraMode::Returning(saved_transform);
        }
        return;
    }

    if !pressed_e {
        return;
    }
    // Block all interactions while a visitor is in the bunker — the
    // dialogue runs and the player can't escape mid-conversation.
    if matches!(*visitor_state, VisitorState::Inside { .. }) {
        return;
    }

    let pick = pick_interaction(&camera_q, &laptop_q, &button_q, &cctv_q, &visitor_state);

    match pick {
        Some(Interaction::Door) => {
            admit.write(AdmitVisitor);
        }
        Some(Interaction::Laptop) => {
            *next_state = NextState::Pending(PlayingState::Laptop);
        }
        Some(Interaction::Cctv) => {
            if let Ok(cam_t) = camera_q.single() {
                *camera_mode = CameraMode::AtCctv {
                    saved_transform: *cam_t,
                };
            }
        }
        None => {}
    }
}

/// Build the in-range candidate list and pick the one the player is
/// most facing. Returns `None` if nothing is in range.
///
/// Pure proximity is the gate (no facing requirement when only one
/// thing is nearby), but when several interactables overlap — the
/// laptop and the door button sit on the same desk, the CCTV
/// monitor is just above — the highest forward-dot-product wins.
fn pick_interaction(
    camera_q: &Query<&Transform, With<FpsCamera>>,
    laptop_q: &Query<&Transform, With<LaptopObject>>,
    button_q: &Query<&Transform, With<DoorButton>>,
    cctv_q: &Query<&Transform, With<CctvMonitor>>,
    visitor_state: &VisitorState,
) -> Option<Interaction> {
    let cam = camera_q.single().ok()?;
    let cam_pos = cam.translation;
    let cam_forward = cam.forward().as_vec3();

    let mut best: Option<(Interaction, f32)> = None;
    let mut consider = |kind: Interaction, target: Vec3| {
        let to_target = target - cam_pos;
        if to_target.length() > INTERACT_DIST {
            return;
        }
        let dir = to_target.normalize_or_zero();
        let dot = cam_forward.dot(dir);
        // Slight bias toward the front of the player so the cone
        // matches the camera frustum — anything behind the camera
        // is excluded even if it's in proximity range.
        if dot < -0.2 {
            return;
        }
        if best.is_none_or(|(_, d)| dot > d) {
            best = Some((kind, dot));
        }
    };

    // Door button is only a valid interaction while a visitor is
    // knocking. Otherwise it's a dim dome with no purpose.
    if matches!(visitor_state, VisitorState::Knocking { .. })
        && let Ok(button) = button_q.single()
    {
        consider(Interaction::Door, button.translation);
    }
    if let Ok(laptop) = laptop_q.single() {
        consider(Interaction::Laptop, laptop.translation);
    }
    if let Ok(cctv) = cctv_q.single() {
        consider(Interaction::Cctv, cctv.translation);
    }

    best.map(|(kind, _)| kind)
}
