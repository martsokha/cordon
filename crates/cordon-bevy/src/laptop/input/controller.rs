//! Raw input reading: scroll zoom, keyboard pan, drag pan, edge
//! scroll.
//!
//! All systems write to [`CameraTarget`] and run in
//! [`ControllerSet`]. Follow-awareness: systems that move the
//! camera (keyboard pan, drag pan, edge scroll) *break* follow
//! by snapshotting the followed entity's current transform
//! into `target.position` before clearing the follow slot, so
//! the camera doesn't jump when the player pans away from a
//! followed NPC. Zoom does *not* break follow — it anchors the
//! zoom around the followed entity instead of the cursor.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use super::{CameraTarget, MAP_BOUND, ZOOM_MAX, ZOOM_MIN, snapshot_follow};
use crate::PlayingState;
use crate::laptop::ui::LaptopTab;

const ZOOM_SENSITIVITY: f32 = 0.12;
const PAN_SPEED: f32 = 300.0;

/// Minimum cursor displacement (pixels) before a left-mouse
/// press engages drag-pan. Below this, the click is treated as
/// a primary action (NPC selection, area tooltip, etc.) and
/// drag-pan is a no-op. Matches the macOS Cocoa drag
/// threshold; Windows uses ~5.
const DRAG_ENGAGE_THRESHOLD_PX: f32 = 4.0;

/// System set for all controller input systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControllerSet;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (read_zoom, read_keyboard_pan, read_drag_pan)
                .in_set(ControllerSet)
                .run_if(in_state(PlayingState::Laptop))
                .run_if(resource_equals(LaptopTab::Map)),
        );
    }
}

/// Read the camera's current world-space position and its
/// orthographic zoom scale. Returns `(Vec2::ZERO, 1.0)` as a
/// defensive fallback when no camera entity matches — should
/// never happen in practice but keeps the call sites branchless.
fn read_camera(camera_q: &Query<(&Transform, &Projection), With<Camera2d>>) -> (Vec2, f32) {
    let Some((transform, proj)) = camera_q.iter().next() else {
        return (Vec2::ZERO, 1.0);
    };
    let pos = transform.translation.truncate();
    let scale = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    (pos, scale)
}

/// Scroll-wheel zoom.
///
/// Follow-aware:
/// - If nothing is being followed, zoom anchors on the cursor
///   (so the point under the cursor stays visually fixed as
///   the scale changes).
/// - If something is being followed, zoom ignores the cursor
///   and anchors on the followed entity. Follow is *not*
///   broken — the player stays locked onto whoever they
///   picked.
fn read_zoom(
    mut scroll: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
) {
    let delta: f32 = scroll.read().map(|e| e.y).sum();
    if delta == 0.0 {
        return;
    }

    let factor = 1.0 - delta * ZOOM_SENSITIVITY;
    let new_zoom = (target.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);

    // Only the cursor-anchor path mutates `target.position`.
    // Follow mode leaves the position alone because
    // `apply_camera` reads the followed entity's transform
    // directly and ignores `target.position` while following.
    if target.following.is_none()
        && let Some(cursor_screen) = windows.single().ok().and_then(|w| w.cursor_position())
        && let Some((camera, cam_transform)) = cameras.iter().next()
        && let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen)
    {
        let zoom_ratio = 1.0 - new_zoom / target.zoom;
        let offset = (cursor_world - target.position) * zoom_ratio;
        target.position += offset;
        target.position = target.position.clamp(-MAP_BOUND, MAP_BOUND);
    }

    target.zoom = new_zoom;
}

/// WASD / arrow-key pan. Breaks any active follow: snapshots
/// the followed entity's position into `target.position` first
/// so the camera continues from where it was, not from a stale
/// stored position.
fn read_keyboard_pan(
    keys: Res<ButtonInput<KeyCode>>,
    // Real time, not virtual: camera pan speed must not
    // accelerate with the sim time cheat. 1× and 64× should
    // both feel the same when the player nudges WASD.
    time: Res<Time<Real>>,
    camera_q: Query<(&Transform, &Projection), With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
) {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    if dir == Vec2::ZERO {
        return;
    }

    let (camera_pos, zoom_scale) = read_camera(&camera_q);
    snapshot_follow(&mut target, camera_pos);

    target.position += dir.normalize() * PAN_SPEED * zoom_scale * time.delta_secs();
    target.position = target.position.clamp(-MAP_BOUND, MAP_BOUND);
}

/// Left-mouse drag-pan.
///
/// Uses a displacement threshold so a click on an NPC dot (or
/// any other primary-click UI) doesn't engage drag-pan on a
/// tiny cursor twitch. Until the cumulative cursor
/// displacement since mouse-down exceeds
/// `DRAG_ENGAGE_THRESHOLD_PX`, the drag is ignored and the
/// click can be interpreted by other systems. Once engaged,
/// the drag remains engaged for the duration of the hold.
///
/// When the drag first engages and follow is active, the
/// followed entity's current transform is snapshotted into
/// `target.position` before the follow slot is cleared — so
/// the camera visually stays on the entity at the exact
/// moment drag starts, then pans from there.
#[derive(Default)]
struct DragState {
    pending: Option<Vec2>,
    engaged: bool,
}

fn read_drag_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    camera_q: Query<(&Transform, &Projection), With<Camera2d>>,
    mut target: ResMut<CameraTarget>,
    mut state: Local<DragState>,
) {
    if !mouse.pressed(MouseButton::Left) {
        // Button released: reset the threshold tracker and
        // drop any pending motion events that accumulated
        // while the button was up.
        state.pending = None;
        state.engaged = false;
        motion.clear();
        return;
    }

    let delta: Vec2 = motion.read().map(|e| e.delta).sum();

    // First frame of a press where the button just became
    // held: seed the pending tracker, don't act yet.
    if mouse.just_pressed(MouseButton::Left) {
        state.pending = Some(Vec2::ZERO);
        state.engaged = false;
    }

    if !state.engaged {
        // Accumulate cursor displacement until we cross the
        // engage threshold. Before that point, drag-pan is a
        // no-op so primary-click handlers (NPC selection,
        // tooltips) can consume the click freely.
        let pending = state.pending.get_or_insert(Vec2::ZERO);
        *pending += delta;
        if pending.length() < DRAG_ENGAGE_THRESHOLD_PX {
            return;
        }
        // Threshold crossed — engage. Snapshot any active
        // follow before we break it, so the camera stays on
        // its *current* lerped position instead of jumping
        // to wherever `target.position` was last stored.
        let (camera_pos, _) = read_camera(&camera_q);
        snapshot_follow(&mut target, camera_pos);
        state.engaged = true;
    }

    if delta == Vec2::ZERO {
        return;
    }

    let (_, zoom_scale) = read_camera(&camera_q);
    target.position -= Vec2::new(delta.x, -delta.y) * zoom_scale;
    target.position = target.position.clamp(-MAP_BOUND, MAP_BOUND);
}
