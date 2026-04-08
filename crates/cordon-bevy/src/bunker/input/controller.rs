//! FPS camera controls: mouse look, WASD movement.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use super::FpsCamera;
use crate::PlayingState;
use crate::bunker::{CameraMode, VisitorState};

const MOVE_SPEED: f32 = 4.0;
const LOOK_SENSITIVITY: f32 = 0.003;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (fps_look, fps_move)
                .run_if(in_state(PlayingState::Bunker))
                .run_if(|mode: Res<CameraMode>| matches!(*mode, CameraMode::Free))
                // Freeze look/move only once a visitor is `Inside`
                // and dialogue is running. While `Knocking` the
                // player should still be able to walk to the desk.
                .run_if(|state: Res<VisitorState>| !matches!(*state, VisitorState::Inside { .. })),
        );
    }
}

fn fps_look(
    mut motion: MessageReader<MouseMotion>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let delta: Vec2 = motion.read().map(|e| e.delta).sum();
    if delta == Vec2::ZERO {
        return;
    }

    for mut transform in &mut camera_q {
        let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        yaw -= delta.x * LOOK_SENSITIVITY;
        pitch -= delta.y * LOOK_SENSITIVITY;
        pitch = pitch.clamp(-1.4, 1.4);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

fn fps_move(
    keys: Res<ButtonInput<KeyCode>>,
    // Player walking speed must stay real-time so fast-forwarding
    // the sim doesn't also rocket the player around the bunker.
    time: Res<Time<Real>>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }
    if input == Vec2::ZERO {
        return;
    }

    for mut transform in &mut camera_q {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let movement = (flat_forward * input.y + flat_right * input.x).normalize_or_zero()
            * MOVE_SPEED
            * time.delta_secs();
        transform.translation += movement;
        // Player stays on their side: behind trade grate (z < 1.3), within walls
        transform.translation.x = transform.translation.x.clamp(-1.8, 1.8);
        transform.translation.z = transform.translation.z.clamp(-4.7, 1.3);
    }
}
