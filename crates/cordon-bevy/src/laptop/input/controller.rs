//! Raw input reading: scroll zoom, keyboard pan, drag pan, edge scroll.
//!
//! All systems write to [`CameraTarget`] and run in [`ControllerSet`].

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use super::{CameraTarget, ZOOM_MAX, ZOOM_MIN};
use crate::AppState;

const ZOOM_SENSITIVITY: f32 = 0.12;
const PAN_SPEED: f32 = 300.0;
const EDGE_PAN_MARGIN: f32 = 20.0;
const EDGE_PAN_SPEED: f32 = 200.0;

/// System set for all controller input systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControllerSet;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (read_zoom, read_keyboard_pan, read_drag_pan, read_edge_pan)
                .in_set(ControllerSet)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

fn current_zoom_scale(camera_q: &Query<&Projection, With<Camera2d>>) -> f32 {
    camera_q
        .iter()
        .next()
        .and_then(|p| match p {
            Projection::Orthographic(o) => Some(o.scale),
            _ => None,
        })
        .unwrap_or(1.0)
}

fn read_zoom(
    mut scroll: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
) {
    let delta: f32 = scroll.read().map(|e| e.y).sum();
    if delta == 0.0 {
        return;
    }

    let factor = 1.0 - delta * ZOOM_SENSITIVITY;
    let new_zoom = (target.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);

    if let Some(cursor_screen) = windows.single().ok().and_then(|w| w.cursor_position()) {
        if let Some((camera, cam_transform, _)) = cameras.iter().next() {
            if let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) {
                let zoom_ratio = 1.0 - new_zoom / target.zoom;
                let offset = (cursor_world - target.position) * zoom_ratio;
                target.position += offset;
            }
        }
    }

    target.zoom = new_zoom;
    target.following = None;
}

fn read_keyboard_pan(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera_q: Query<&Projection, With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir == Vec2::ZERO {
        return;
    }

    let zoom_scale = current_zoom_scale(&camera_q);
    target.position += dir.normalize() * PAN_SPEED * zoom_scale * time.delta_secs();
    target.following = None;
}

fn read_drag_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    camera_q: Query<&Projection, With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
) {
    if !mouse.pressed(MouseButton::Left) {
        motion.clear();
        return;
    }

    let delta: Vec2 = motion.read().map(|e| e.delta).sum();
    if delta == Vec2::ZERO {
        return;
    }

    let zoom_scale = current_zoom_scale(&camera_q);
    target.position -= Vec2::new(delta.x, -delta.y) * zoom_scale;
    target.following = None;
}

fn read_edge_pan(
    windows: Query<&Window>,
    time: Res<Time>,
    camera_q: Query<&Projection, With<Camera2d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut target: ResMut<CameraTarget>,
) {
    if mouse.pressed(MouseButton::Left) {
        return;
    }
    let Some(window) = windows.single().ok() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    let w = window.width();
    let h = window.height();
    let mut dir = Vec2::ZERO;

    if cursor.x < EDGE_PAN_MARGIN {
        dir.x -= 1.0;
    }
    if cursor.x > w - EDGE_PAN_MARGIN {
        dir.x += 1.0;
    }
    if cursor.y < EDGE_PAN_MARGIN {
        dir.y += 1.0;
    }
    if cursor.y > h - EDGE_PAN_MARGIN {
        dir.y -= 1.0;
    }

    if dir == Vec2::ZERO {
        return;
    }

    let zoom_scale = current_zoom_scale(&camera_q);
    target.position += dir.normalize() * EDGE_PAN_SPEED * zoom_scale * time.delta_secs();
    target.following = None;
}
