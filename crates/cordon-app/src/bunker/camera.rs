use bevy::prelude::*;
use bevy::window::CursorOptions;

use super::resources::*;

#[derive(Component)]
pub struct FpsCamera;

pub(super) fn start_free_look(
    mut mode: ResMut<CameraMode>,
    mut cursor_q: Query<&mut CursorOptions>,
) {
    let saved = match &*mode {
        CameraMode::AtLaptop { saved_transform } => Some(*saved_transform),
        _ => None,
    };
    if let Some(t) = saved {
        *mode = CameraMode::Returning(t);
        for mut cursor in &mut cursor_q {
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

pub(super) fn animate_camera(
    time: Res<Time<Real>>,
    mut mode: ResMut<CameraMode>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let dt = time.delta_secs();
    let factor = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();

    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    match mode.clone() {
        CameraMode::Free => {}
        // The laptop/CCTV transitions are hidden under the fade
        // overlay (see `bunker/fade.rs`), so `Returning` snaps
        // back to the saved transform instantly — any lerp here
        // would just be motion the player never sees. Cursor
        // state is owned by `start_free_look` (OnEnter) and the
        // CCTV audio path, so this arm doesn't touch it.
        CameraMode::Returning(saved) => {
            *transform = saved;
            *mode = CameraMode::Free;
        }
        CameraMode::AtLaptop { .. } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;
            transform.translation = LAPTOP_VIEW_POS;
            transform.rotation = target_rot;
        }
        // Visitor gaze rotation — this is a separate "turn to
        // face the door" effect, not the laptop/CCTV animation,
        // so it keeps its slerp.
        CameraMode::LookingAt { target, .. } => {
            let target_rot = Transform::from_translation(transform.translation)
                .looking_at(target, Vec3::Y)
                .rotation;
            transform.rotation = transform.rotation.slerp(target_rot, factor);
        }
        CameraMode::AtCctv { .. } => {}
    }
}

pub(super) fn enable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = true;
    }
}
