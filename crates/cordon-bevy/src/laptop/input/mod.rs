//! Camera state, smoothing, and follow logic.
//!
//! [`InputPlugin`] owns the [`CameraTarget`] resource and the
//! `apply_camera` system that smoothly interpolates toward it.
//! The [`ControllerPlugin`] reads raw input and writes to the
//! target.
//!
//! # Follow semantics
//!
//! [`CameraTarget::following`] is the "lock camera onto this
//! entity" slot. `apply_camera` reads it every frame and lerps
//! the camera toward the entity's transform, ignoring
//! `target.position` entirely while following is active.
//!
//! Camera-moving controllers (keyboard pan, drag pan, edge
//! scroll) *break* follow by first calling
//! [`snapshot_follow`] — which copies the followed entity's
//! current position into `target.position` and clears
//! `following` — so the camera continues smoothly from
//! exactly where it was. Without the snapshot the camera
//! would jump to a stale `target.position` from whenever
//! follow was last inactive.
//!
//! Zoom does *not* break follow: scrolling while following an
//! NPC anchors the zoom on that NPC instead of on the cursor.

pub mod controller;

use bevy::prelude::*;

use crate::PlayingState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraTarget::default());
        app.insert_resource(EdgeScrollEnabled::default());
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

/// Whether edge-scroll panning is enabled. Off by default —
/// some players find edge-scroll distracting, and the
/// default-off state also means existing test workflows that
/// rest the cursor near the viewport rim don't accidentally
/// drift the camera.
///
/// Toggled via a settings menu (future work) or via dev
/// cheats. The controller system reads this each frame so
/// flipping it at runtime takes effect immediately.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct EdgeScrollEnabled(pub bool);

/// Orthographic-projection scale range. Lower = zoomed in
/// (smaller world area visible), higher = zoomed out.
///
/// `ZOOM_MIN = 0.85` is the zoom-in limit and deliberately
/// not tighter — we don't want the player drilling past
/// NPC-scale detail. `ZOOM_MAX = 2.4` is the zoom-out limit,
/// widened from the original 1.8 so the player can take in a
/// meaningful slice of the map at once (~50% at max zoom-out)
/// without losing the sense of scale that full-map visibility
/// would strip away.
pub const ZOOM_MIN: f32 = 0.85;
pub const ZOOM_MAX: f32 = 2.4;

/// Half-extent of the playable map in world units. Used as
/// the clamp for both the camera transform and the
/// `target.position` state.
const HALF_MAP: f32 = 2500.0;

/// Vec2 form of [`HALF_MAP`] for the pan systems' clamp calls.
/// Visible to the `controller` child module via normal
/// parent-child visibility.
const MAP_BOUND: Vec2 = Vec2::splat(HALF_MAP);

/// Camera smoothing tightness. Closer to 0 = tighter (more
/// of the gap closed per frame); closer to 1 = lazier. 0.05
/// reads as "camera follows crisply" without being snappy.
///
/// Used through [`smooth_factor`] which interprets the base
/// exponentially so the curve is framerate-independent.
const SMOOTHNESS: f32 = 0.05;

fn smooth_factor(dt: f32) -> f32 {
    1.0 - SMOOTHNESS.powi(7).powf(dt)
}

/// If something is being followed, copy the *camera's*
/// current transform into `target.position` and clear
/// `following`. Idempotent: a no-op when nothing is being
/// followed.
///
/// Snapshotting the camera (rather than the followed
/// entity) is deliberate. The camera lags its target by a
/// few frames of lerp; when the player pans away, they
/// expect continuity from what they were just *seeing*, not
/// from where the target actually is. Using the entity
/// position here causes a visible hitch if the player was
/// mid-catch-up when they started panning.
///
/// Call this *before* mutating `target.position` in any
/// camera-moving system — otherwise the camera will snap
/// from the entity's lerped position to whatever stale
/// coordinate `target.position` happened to hold.
fn snapshot_follow(target: &mut CameraTarget, camera_pos: Vec2) {
    if target.following.is_none() {
        return;
    }
    target.position = camera_pos;
    target.following = None;
}

fn apply_camera(
    // Camera smoothing reads *real* time so it stays identical
    // regardless of the player's sim time scale. Without this,
    // pressing F4 to fast-forward the sim would also
    // fast-forward the camera lerp and feel wrong.
    time: Res<Time<Real>>,
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

    for (mut transform, mut proj) in &mut camera_q {
        let current = transform.translation.truncate();
        let new_pos = current.lerp(goal, factor).clamp(-MAP_BOUND, MAP_BOUND);
        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;

        if let Projection::Orthographic(ref mut ortho) = *proj {
            ortho.scale = ortho.scale + (target.zoom - ortho.scale) * factor;
        }
    }
}
