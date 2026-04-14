//! FPS camera controls: mouse look, WASD movement with physics
//! collision via avian3d's `move_and_slide`.

use std::time::Duration;

use avian3d::character_controller::move_and_slide::{
    MoveAndSlide, MoveAndSlideConfig, MoveAndSlideHitResponse,
};
use avian3d::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::PlayingState;
use crate::bunker::components::FpsCamera;
use crate::bunker::resources::{CameraMode, MovementLocked};

const MOVE_SPEED: f32 = 4.0;
const LOOK_SENSITIVITY: f32 = 0.003;

pub(crate) const PLAYER_RADIUS: f32 = 0.3;
pub(crate) const PLAYER_HEIGHT: f32 = 1.0;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (fps_look, fps_move)
                .run_if(in_state(PlayingState::Bunker))
                .run_if(|mode: Res<CameraMode>| matches!(*mode, CameraMode::Free))
                .run_if(not(resource_exists::<MovementLocked>)),
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
    time: Res<Time<Real>>,
    move_and_slide: MoveAndSlide,
    mut camera_q: Query<(Entity, &Collider, &mut Transform), With<FpsCamera>>,
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

    for (entity, collider, mut transform) in &mut camera_q {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let velocity =
            (flat_forward * input.y + flat_right * input.x).normalize_or_zero() * MOVE_SPEED;

        let dt = Duration::from_secs_f32(time.delta_secs());
        let config = MoveAndSlideConfig::default();
        let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);

        let output = move_and_slide.move_and_slide(
            collider,
            transform.translation,
            transform.rotation,
            velocity,
            dt,
            &config,
            &filter,
            |_hit| MoveAndSlideHitResponse::Accept,
        );

        transform.translation = output.position;
        transform.translation.y = 1.6;
    }
}
