//! Generic interaction system: any entity with an [`Interactable`]
//! component can be activated by the player pressing E. The system
//! picks the best candidate (proximity + facing tiebreaker) and fires
//! an [`Interact`] event on the winning entity, which that entity's
//! observer handles.

use bevy::prelude::*;

use super::camera::FpsCamera;
use super::resources::{CameraMode, InteractionLocked};
use crate::locale::L10n;

/// Marker on the interaction prompt UI container.
#[derive(Component)]
pub struct InteractPrompt;

const INTERACT_DIST: f32 = 3.5;

/// Minimum dot product between camera forward and the direction
/// to a candidate. 0.7 ≈ 45° half-angle cone — generous enough
/// to reach top shelves when looking up, tight enough to not
/// grab things behind or beside the player.
const MIN_AIM_DOT: f32 = 0.7;

/// Attach to any entity the player can interact with via E.
///
/// `key` is a Fluent localisation key (e.g. `"interact-laptop"`).
/// The displayed prompt is resolved through [`L10n`] at
/// render time, falling back to the raw key when no translation
/// exists.
#[derive(Component)]
pub struct Interactable {
    pub key: String,
    pub enabled: bool,
}

/// Fired on the winning entity when the player presses E.
#[derive(EntityEvent)]
pub struct Interact {
    pub entity: Entity,
}

pub(super) fn update_prompt(
    camera_q: Query<&Transform, With<FpsCamera>>,
    interactables: Query<(Entity, &GlobalTransform, &Interactable)>,
    camera_mode: Res<CameraMode>,
    l10n: L10n,
    mut prompt_q: Query<(&Children, &mut Visibility), With<InteractPrompt>>,
    mut text_q: Query<&mut Text>,
) {
    if matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        let resolved = l10n.get("interact-exit-camera");
        for (children, mut vis) in &mut prompt_q {
            if let Some(&child) = children.first() {
                if let Ok(mut t) = text_q.get_mut(child) {
                    t.0 = resolved.clone();
                }
            }
            *vis = Visibility::Visible;
        }
        return;
    }

    let best = pick_best(&camera_q, &interactables);

    for (children, mut vis) in &mut prompt_q {
        match best {
            Some((_, interactable)) => {
                if let Some(&child) = children.first() {
                    if let Ok(mut t) = text_q.get_mut(child) {
                        t.0 = l10n.get(&interactable.key);
                    }
                }
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

pub(super) fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    interactables: Query<(Entity, &GlobalTransform, &Interactable)>,
    locked: Option<Res<InteractionLocked>>,
    mut camera_mode: ResMut<CameraMode>,
    mut commands: Commands,
) {
    let pressed_e = keys.just_pressed(KeyCode::KeyE);
    let pressed_esc = keys.just_pressed(KeyCode::Escape);

    if let CameraMode::AtCctv { saved_transform } = *camera_mode {
        if pressed_e || pressed_esc {
            *camera_mode = CameraMode::Returning(saved_transform);
        }
        return;
    }

    if !pressed_e || locked.is_some() {
        return;
    }

    let Some((entity, _)) = pick_best(&camera_q, &interactables) else {
        return;
    };
    commands.trigger(Interact { entity });
}

fn pick_best<'a>(
    camera_q: &Query<&Transform, With<FpsCamera>>,
    interactables: &'a Query<(Entity, &GlobalTransform, &Interactable)>,
) -> Option<(Entity, &'a Interactable)> {
    let cam = camera_q.single().ok()?;
    let cam_pos = cam.translation;
    let cam_forward = cam.forward().as_vec3();

    let mut best: Option<(Entity, &Interactable, f32)> = None;
    for (entity, gt, interactable) in interactables.iter() {
        if !interactable.enabled {
            continue;
        }
        let to_target = gt.translation() - cam_pos;
        let dist = to_target.length();
        if !(0.01..=INTERACT_DIST).contains(&dist) {
            continue;
        }
        let dir = to_target.normalize_or_zero();
        let dot = cam_forward.dot(dir);
        if dot < MIN_AIM_DOT {
            continue;
        }
        // Among candidates in the aiming cone, pick the one
        // most aligned with the crosshair (highest dot). This
        // means the player's gaze direction decides which slot
        // wins when multiple are within range.
        if best.as_ref().is_none_or(|(_, _, d)| dot > *d) {
            best = Some((entity, interactable, dot));
        }
    }
    best.map(|(e, i, _)| (e, i))
}
