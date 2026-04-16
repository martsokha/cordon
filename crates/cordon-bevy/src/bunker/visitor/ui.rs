//! Visitor UI feedback: button glow, button enabled state, cursor
//! lock, and the door-button interaction observer.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::state::{AdmitVisitor, VisitorState};
use crate::bunker::components::DoorButton;
use crate::bunker::interaction::{Interact, Interactable};

pub(super) fn update_button_glow(
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

pub(super) fn update_cursor_lock(
    state: Res<VisitorState>,
    mut cursor_q: Query<&mut CursorOptions>,
) {
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

pub(super) fn update_button_enabled(
    visitor_state: Res<VisitorState>,
    mut buttons: Query<&mut Interactable, With<DoorButton>>,
) {
    let active = matches!(*visitor_state, VisitorState::Knocking { .. });
    for mut i in &mut buttons {
        i.enabled = active;
    }
}

pub(super) fn attach_door_observer(
    mut commands: Commands,
    new_buttons: Query<Entity, Added<DoorButton>>,
) {
    for entity in &new_buttons {
        commands.entity(entity).observe(
            |_trigger: On<Interact>, mut admit: MessageWriter<AdmitVisitor>| {
                admit.write(AdmitVisitor);
            },
        );
    }
}
