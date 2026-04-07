//! Camera state, smoothing, and follow logic.
//!
//! [`InputPlugin`] owns the [`CameraTarget`] resource and the
//! `apply_camera` system that smoothly interpolates toward it.
//! The [`ControllerPlugin`] reads raw input and writes to the target.

pub mod controller;

use bevy::prelude::*;

use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraTarget::default());
        app.add_plugins(controller::ControllerPlugin);
        app.add_systems(
            Update,
            apply_camera
                .after(controller::ControllerSet)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// The target state the camera smoothly interpolates toward.
#[derive(Resource)]
pub struct CameraTarget {
    pub position: Vec2,
    pub zoom: f32,
    pub following: Option<Entity>,
}

impl Default for CameraTarget {
    fn default() -> Self {
        Self {
            position: Vec2::new(0.0, -100.0),
            zoom: 1.0,
            following: None,
        }
    }
}

pub const ZOOM_MIN: f32 = 0.85;
pub const ZOOM_MAX: f32 = 1.8;

const SMOOTHNESS: f32 = 0.3;

fn smooth_factor(dt: f32) -> f32 {
    1.0 - SMOOTHNESS.powi(7).powf(dt)
}

fn apply_camera(
    time: Res<Time>,
    target: Res<CameraTarget>,
    transforms: Query<&Transform, Without<Camera2d>>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let dt = time.delta_secs();
    let factor = smooth_factor(dt);

    let goal = if let Some(entity) = target.following {
        transforms
            .get(entity)
            .map(|t| t.translation.truncate())
            .unwrap_or(target.position)
    } else {
        target.position
    };

    let half_map = 2500.0;
    for (mut transform, mut proj) in &mut camera_q {
        let current = transform.translation.truncate();
        let new_pos = current
            .lerp(goal, factor)
            .clamp(Vec2::splat(-half_map), Vec2::splat(half_map));
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;

        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = ortho.scale + (target.zoom - ortho.scale) * factor;
        }
    }
}
