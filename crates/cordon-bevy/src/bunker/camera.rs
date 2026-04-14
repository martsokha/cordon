use bevy::prelude::*;

use super::components::*;
use super::resources::*;

pub(super) fn start_laptop_zoom(
    camera_q: Query<&Transform, With<FpsCamera>>,
    mut mode: ResMut<CameraMode>,
) {
    if let Ok(transform) = camera_q.single() {
        *mode = CameraMode::ZoomingToLaptop {
            saved_transform: *transform,
        };
    }
}

pub(super) fn start_free_look(
    mut mode: ResMut<CameraMode>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let saved = match &*mode {
        CameraMode::AtLaptop { saved_transform }
        | CameraMode::ZoomingToLaptop { saved_transform } => Some(*saved_transform),
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
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let dt = time.delta_secs();
    let factor = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();

    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    match mode.clone() {
        CameraMode::Free => {}
        CameraMode::Returning(saved) => {
            transform.translation = transform.translation.lerp(saved.translation, factor);
            transform.rotation = transform.rotation.slerp(saved.rotation, factor);
            let pos_done = transform.translation.distance(saved.translation) < 0.01;
            let rot_done = transform.rotation.dot(saved.rotation).abs() > 0.9999;
            if pos_done && rot_done {
                *transform = saved;
                *mode = CameraMode::Free;
            }
        }
        CameraMode::ZoomingToLaptop { saved_transform } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;

            transform.translation = transform.translation.lerp(LAPTOP_VIEW_POS, factor);
            transform.rotation = transform.rotation.slerp(target_rot, factor);

            if transform.translation.distance(LAPTOP_VIEW_POS) < 0.01 {
                *mode = CameraMode::AtLaptop { saved_transform };
                for mut cursor in &mut cursor_q {
                    cursor.grab_mode = bevy::window::CursorGrabMode::None;
                    cursor.visible = true;
                }
            }
        }
        CameraMode::AtLaptop { .. } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;
            transform.translation = LAPTOP_VIEW_POS;
            transform.rotation = target_rot;
        }
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
