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
        app.add_systems(OnExit(PlayingState::Bunker), disable_bunker_camera);
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
const CAMERA_LERP_SPEED: f32 = 5.0;

#[derive(Resource, Clone)]
enum CameraMode {
    Free,
    ZoomingToLaptop { saved_transform: Transform },
    AtLaptop { saved_transform: Transform },
    Returning,
}

fn start_laptop_zoom(camera_q: Query<&Transform, With<FpsCamera>>, mut mode: ResMut<CameraMode>) {
    if let Ok(transform) = camera_q.single() {
        *mode = CameraMode::ZoomingToLaptop {
            saved_transform: *transform,
        };
    }
}

fn start_free_look(mut mode: ResMut<CameraMode>) {
    match &*mode {
        CameraMode::AtLaptop { .. } | CameraMode::ZoomingToLaptop { .. } => {
            *mode = CameraMode::Returning;
        }
        _ => {}
    }
}

fn animate_camera(
    time: Res<Time>,
    mut mode: ResMut<CameraMode>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let dt = time.delta_secs();
    let factor = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();

    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    match mode.clone() {
        CameraMode::Free | CameraMode::Returning => {
            if matches!(*mode, CameraMode::Returning) {
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

fn disable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = false;
    }
}
