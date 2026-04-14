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

/// Distance (in metres) the player walks between consecutive
/// footstep events. ~0.7 m matches a natural stride at
/// [`MOVE_SPEED`] = 4 m/s.
const STEP_DISTANCE: f32 = 0.7;

pub(crate) const PLAYER_RADIUS: f32 = 0.3;
pub(crate) const PLAYER_HEIGHT: f32 = 1.0;

/// Fired whenever the player walks far enough for a new footstep.
/// `pos` is the world-space floor position under the camera.
/// Consumed by the bunker particle system to scuff a dust puff.
#[derive(Message, Debug, Clone, Copy)]
pub struct FootstepScuffed {
    pub pos: Vec3,
}

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FootstepScuffed>();
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
    mut distance_since_step: Local<f32>,
    mut footsteps: MessageWriter<FootstepScuffed>,
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
        // Reset the accumulator so stopping and starting again
        // doesn't fire a stale step the moment the player moves.
        *distance_since_step = 0.0;
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

        let before = transform.translation;
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

        // Accumulate horizontal distance actually travelled (not
        // the velocity intent) so collisions + wall-slide naturally
        // slow the step cadence. Fire a footstep at ground level
        // when the accumulator crosses STEP_DISTANCE.
        let delta = (transform.translation - before).xz().length();
        *distance_since_step += delta;
        if *distance_since_step >= STEP_DISTANCE {
            *distance_since_step = 0.0;
            footsteps.write(FootstepScuffed {
                pos: Vec3::new(transform.translation.x, 0.0, transform.translation.z),
            });
        }
    }
}
