//! Visitor lifecycle: queue, knocking, admit, dialogue, dismiss.
//!
//! A [`Visitor`] is a small description (faction, name, yarn start
//! node) waiting in the [`VisitorQueue`]. The state machine
//! ([`VisitorState`]) cycles through:
//!
//! - `Quiet` — no one at the door, button is dim, queue may be empty
//! - `Knocking` — head of queue has arrived, door button glows red
//! - `Inside` — player pressed E to admit them: sprite spawned,
//!   dialogue running. The state stays here until [`CurrentDialogue`]
//!   returns to `Idle`, at which point the sprite is despawned and
//!   we drop back to `Quiet`.
//!
//! Player input is handled in [`super::input`]: while the camera is
//! near the desk and a visitor is `Knocking`, pressing **E** sends
//! an [`AdmitVisitor`] message which this module's
//! [`apply_admit_visitor`] system handles. Dialogue start is in
//! turn delegated to the dialogue module via a `StartDialogue`
//! message — visitor never touches the yarn runner directly.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;

use super::dialogue::{CurrentDialogue, StartDialogue};
use super::{CameraMode, DoorButton, FpsCamera};
use crate::PlayingState;

pub struct VisitorPlugin;

impl Plugin for VisitorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VisitorQueue::default());
        app.insert_resource(VisitorState::Quiet);
        app.add_message::<AdmitVisitor>();
        app.add_systems(
            Update,
            (
                arrive_next_visitor,
                apply_admit_visitor,
                update_button_glow,
                update_cursor_lock,
                dismiss_on_dialogue_complete,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}

/// A pending visitor: who they are and what yarn node to start when
/// the player admits them.
#[derive(Debug, Clone)]
pub struct Visitor {
    pub display_name: String,
    pub faction: Id<Faction>,
    pub yarn_node: String,
}

/// FIFO queue of visitors waiting outside.
#[derive(Resource, Default, Debug)]
pub struct VisitorQueue(pub VecDeque<Visitor>);

/// Current door state. Drives the button visual, sprite spawning,
/// and camera lock.
#[derive(Resource, Debug, Clone)]
pub enum VisitorState {
    /// No one at the door. The button is dim.
    Quiet,
    /// A visitor is waiting outside. The button glows red.
    Knocking { visitor: Visitor },
    /// Player admitted the visitor. Sprite is spawned and dialogue
    /// runner is on a yarn node.
    Inside { visitor: Visitor, sprite: Entity },
}

/// Sent by the bunker `interact` system when the player presses E
/// while a visitor is knocking. Handled by [`apply_admit_visitor`].
#[derive(Message, Debug, Default, Clone, Copy)]
pub struct AdmitVisitor;

/// Marker for the visitor sprite entity, so we can despawn it when
/// the dialogue ends.
#[derive(Component)]
struct VisitorSprite;

/// World-space position of the placeholder visitor sprite — also
/// the point the camera turns to face during dialogue.
const VISITOR_SPRITE_POS: Vec3 = Vec3::new(0.0, 1.2, 2.4);

/// When the door is quiet and the queue is non-empty, pop the next
/// visitor and transition to Knocking.
fn arrive_next_visitor(mut state: ResMut<VisitorState>, mut queue: ResMut<VisitorQueue>) {
    if !matches!(*state, VisitorState::Quiet) {
        return;
    }
    if let Some(visitor) = queue.0.pop_front() {
        info!("visitor arrived: {}", visitor.display_name);
        *state = VisitorState::Knocking { visitor };
    }
}

/// Swap the door-button material's emissive based on state. Saves
/// us creating per-frame materials by mutating the existing handle
/// in place — there's only one button so the cost is negligible.
fn update_button_glow(
    state: Res<VisitorState>,
    button_q: Query<&MeshMaterial3d<StandardMaterial>, With<DoorButton>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !state.is_changed() {
        return;
    }
    let Ok(mat_handle) = button_q.single() else {
        return;
    };
    let Some(mat) = materials.get_mut(&mat_handle.0) else {
        return;
    };
    mat.emissive = match *state {
        VisitorState::Knocking { .. } => LinearRgba::new(2.0, 0.05, 0.05, 1.0),
        _ => LinearRgba::BLACK,
    };
}

/// Handle [`AdmitVisitor`] messages: spawn the visitor sprite, turn
/// the camera, and ask the dialogue module to start the yarn node.
/// Drains all pending messages but only acts on the first one if
/// state is `Knocking` — extra admits are no-ops.
#[allow(clippy::too_many_arguments)]
fn apply_admit_visitor(
    mut commands: Commands,
    mut requests: MessageReader<AdmitVisitor>,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    mut start_dialogue: MessageWriter<StartDialogue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if requests.read().next().is_none() {
        return;
    }
    let visitor = match &*state {
        VisitorState::Knocking { visitor } => visitor.clone(),
        _ => return,
    };

    // Placeholder sprite: a vertical colored quad standing in front
    // of the desk. Real visitor art replaces this later.
    let sprite_color = match visitor.faction.as_str() {
        "garrison" => Color::srgb(0.42, 0.55, 0.30),
        "syndicate" => Color::srgb(0.66, 0.27, 0.16),
        "institute" => Color::srgb(0.23, 0.55, 0.62),
        "devoted" => Color::srgb(0.48, 0.25, 0.55),
        "drifters" => Color::srgb(0.62, 0.48, 0.31),
        _ => Color::srgb(0.6, 0.6, 0.6),
    };
    let sprite_entity = commands
        .spawn((
            VisitorSprite,
            Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.4, 0.9)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: sprite_color,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_translation(VISITOR_SPRITE_POS)
                .looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
        ))
        .id();

    // Turn the camera (rotation only) to face the visitor. Save the
    // current transform so we can restore on dismissal.
    if let Ok(cam_t) = camera_q.single() {
        *camera_mode = CameraMode::LookingAt {
            target: VISITOR_SPRITE_POS,
            saved_transform: *cam_t,
        };
    }

    // Hand off the actual yarn-node start to the dialogue module so
    // visitor never touches the runner directly.
    start_dialogue.write(StartDialogue {
        node: visitor.yarn_node.clone(),
    });

    info!("visitor admitted: {}", visitor.display_name);
    *state = VisitorState::Inside {
        visitor,
        sprite: sprite_entity,
    };
}

/// When dialogue *transitions* from active back to Idle and we're
/// still in `Inside`, despawn the sprite, turn the camera back to
/// where it was, and return to `Quiet` so the next visitor (if any)
/// can arrive.
///
/// The transition check is critical: `CurrentDialogue` is `Idle` at
/// startup *and* between dialogues, so a naive `if Idle` check fires
/// the same frame `apply_admit_visitor` ran (yarn hasn't ticked yet,
/// so the resource still reads Idle). We track the previous active
/// state in a `Local` and only dismiss on the falling edge.
fn dismiss_on_dialogue_complete(
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    current: Res<CurrentDialogue>,
    mut was_active: Local<bool>,
) {
    let now_active = !matches!(*current, CurrentDialogue::Idle);
    let just_ended = *was_active && !now_active;
    *was_active = now_active;

    if !just_ended {
        return;
    }
    if let VisitorState::Inside { visitor, sprite } = &*state {
        let name = visitor.display_name.clone();
        let sprite_entity = *sprite;
        commands.entity(sprite_entity).despawn();
        // Slerp the camera back to the saved transform.
        if let CameraMode::LookingAt {
            saved_transform, ..
        } = *camera_mode
        {
            *camera_mode = CameraMode::Returning(saved_transform);
        }
        *state = VisitorState::Quiet;
        info!("visitor dismissed: {name}");
    }
}

/// Show/hide the cursor based on whether the dialogue UI is up.
/// Only the `Inside` state needs a visible cursor (to click choice
/// buttons). `Knocking` keeps full FPS control so the player can
/// still walk to the desk.
fn update_cursor_lock(state: Res<VisitorState>, mut cursor_q: Query<&mut CursorOptions>) {
    if !state.is_changed() {
        return;
    }
    let unlock = matches!(*state, VisitorState::Inside { .. });
    for mut cursor in &mut cursor_q {
        if unlock {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        } else {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}
