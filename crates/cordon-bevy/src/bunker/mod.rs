//! 3D bunker scene with FPS camera. Visitor dialogue lives here too —
//! the player meets visitors at the counter inside the bunker, not on
//! the laptop map.

mod cctv;
mod dialogue;
mod input;
pub mod room;
mod visitor;

use bevy::prelude::*;

pub use self::cctv::{ANTECHAMBER_VISITOR_POS, CctvMonitor};
pub use self::visitor::{Visitor, VisitorQueue, VisitorState};
use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            room::RoomPlugin,
            dialogue::DialoguePlugin,
            visitor::VisitorPlugin,
            cctv::CctvPlugin,
        ));
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
pub struct InteractPrompt;

/// Marker for the small dome on the desk that admits a knocking
/// visitor. Spawned by the room module; clicked via mesh picking.
#[derive(Component)]
pub struct DoorButton;

/// Camera zoomed to laptop. desk_z=1.0, screen at desk_z+0.12=1.12
const LAPTOP_VIEW_POS: Vec3 = Vec3::new(0.0, 1.15, 0.5);
const LAPTOP_VIEW_TARGET: Vec3 = Vec3::new(0.0, 0.90, 1.12);
const CAMERA_LERP_SPEED: f32 = 6.0;

#[derive(Resource, Clone)]
pub enum CameraMode {
    Free,
    ZoomingToLaptop {
        saved_transform: Transform,
    },
    AtLaptop {
        saved_transform: Transform,
    },
    Returning(Transform),
    /// Smoothly turn (rotation only) to face a world-space point.
    /// Used while a visitor is inside the bunker. The position is
    /// untouched — the player stays where they were standing.
    LookingAt {
        target: Vec3,
        saved_transform: Transform,
    },
    /// Player is studying the CCTV feed in fullscreen. The CCTV
    /// camera takes over the window and the FPS camera goes
    /// inactive until the player presses E or Esc.
    AtCctv {
        saved_transform: Transform,
    },
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
    // Bunker camera animation is player-facing, not sim state —
    // use real time so accelerating the sim doesn't speed up the
    // laptop-to-bunker return lerp.
    time: Res<Time<Real>>,
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
            // The visitor-return case only changes rotation (the
            // player never moved), so a translation-only threshold
            // would flip back to Free on the very first frame
            // before the slerp had any visible effect. Check both.
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
        CameraMode::LookingAt { target, .. } => {
            // Rotation only — player stays put. Smoothly slerp the
            // current rotation toward facing the visitor.
            let target_rot = Transform::from_translation(transform.translation)
                .looking_at(target, Vec3::Y)
                .rotation;
            transform.rotation = transform.rotation.slerp(target_rot, factor);
        }
        CameraMode::AtCctv { .. } => {
            // The CCTV camera takes over the window during fullscreen
            // mode. The FPS camera doesn't move; the cctv plugin's
            // `apply_cctv_fullscreen` system handles the swap.
        }
    }
}

fn enable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = true;
    }
}
