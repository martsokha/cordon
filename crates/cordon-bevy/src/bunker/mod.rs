//! 3D bunker scene with FPS camera.

pub mod blockout;
mod input;

use bevy::prelude::*;

use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((input::InputPlugin, blockout::BlockoutPlugin));
        app.insert_resource(CameraMode::Free);
        app.add_systems(OnEnter(PlayingState::Bunker), enable_bunker_camera);
        app.add_systems(OnEnter(PlayingState::Laptop), start_laptop_zoom);
        app.add_systems(OnEnter(PlayingState::Bunker), start_free_look);
        app.add_systems(Update, animate_camera);
    }
}

#[derive(Resource)]
pub struct BunkerSpawned;

#[derive(Component)]
pub struct FpsCamera;

#[derive(Component)]
pub struct LaptopObject;

#[derive(Component)]
pub struct BunkerUi;

#[derive(Component)]
pub struct InteractPrompt;

/// Camera zoomed to laptop. desk_z=1.0, screen at desk_z+0.12=1.12
const LAPTOP_VIEW_POS: Vec3 = Vec3::new(0.0, 1.15, 0.5);
const LAPTOP_VIEW_TARGET: Vec3 = Vec3::new(0.0, 0.90, 1.12);
const CAMERA_LERP_SPEED: f32 = 6.0;

#[derive(Resource, Clone)]
pub enum CameraMode {
    Free,
    ZoomingToLaptop { saved_transform: Transform },
    AtLaptop { saved_transform: Transform },
    Returning(Transform),
}

fn start_laptop_zoom(camera_q: Query<&Transform, With<FpsCamera>>, mut mode: ResMut<CameraMode>) {
    if let Ok(transform) = camera_q.single() {
        *mode = CameraMode::ZoomingToLaptop {
            saved_transform: *transform,
        };
    }
}

fn start_free_look(
    mut mode: ResMut<CameraMode>,
    mut laptop_cam: Query<&mut Camera, With<crate::laptop::LaptopCamera>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let saved = match &*mode {
        CameraMode::AtLaptop { saved_transform }
        | CameraMode::ZoomingToLaptop { saved_transform } => Some(*saved_transform),
        _ => None,
    };
    if let Some(t) = saved {
        *mode = CameraMode::Returning(t);
        for mut cam in &mut laptop_cam {
            cam.is_active = false;
        }
        for mut cursor in &mut cursor_q {
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

fn animate_camera(
    time: Res<Time>,
    mut mode: ResMut<CameraMode>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
    mut laptop_cam: Query<&mut Camera, (With<crate::laptop::LaptopCamera>, Without<FpsCamera>)>,
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
            if transform.translation.distance(saved.translation) < 0.1 {
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
                for mut cam in &mut laptop_cam {
                    cam.is_active = true;
                }
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
    }
}

fn enable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = true;
    }
}
