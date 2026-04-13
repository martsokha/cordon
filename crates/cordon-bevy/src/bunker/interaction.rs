//! Generic interaction system: any entity with an [`Interactable`]
//! component can be activated by the player pressing E. The system
//! picks the best candidate (proximity + facing tiebreaker) and fires
//! an [`Interact`] event on the winning entity, which that entity's
//! observer handles.

use bevy::prelude::*;

use super::components::{FpsCamera, InteractPrompt};
use super::resources::CameraMode;

const INTERACT_DIST: f32 = 3.5;

/// Attach to any entity the player can interact with via E.
#[derive(Component)]
pub struct Interactable {
    pub prompt: &'static str,
    pub enabled: bool,
}

/// When present as a resource, all interactions are blocked. The
/// visitor module inserts this while a visitor is inside the bunker
/// so the player can't escape mid-conversation.
#[derive(Resource)]
pub struct InteractionLocked;

/// Fired on the winning entity when the player presses E.
#[derive(EntityEvent)]
pub struct Interact {
    pub entity: Entity,
}

pub(super) fn update_prompt(
    camera_q: Query<&Transform, With<FpsCamera>>,
    interactables: Query<(Entity, &GlobalTransform, &Interactable)>,
    camera_mode: Res<CameraMode>,
    mut prompt_q: Query<(&mut Text, &mut Visibility), With<InteractPrompt>>,
) {
    if matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        for (mut text, mut vis) in &mut prompt_q {
            text.0 = "[E] Exit Camera".into();
            *vis = Visibility::Visible;
        }
        return;
    }

    let best = pick_best(&camera_q, &interactables);

    for (mut text, mut vis) in &mut prompt_q {
        match best {
            Some((_, interactable)) => {
                text.0 = interactable.prompt.into();
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
        if to_target.length() > INTERACT_DIST {
            continue;
        }
        let dir = to_target.normalize_or_zero();
        let dot = cam_forward.dot(dir);
        if dot < -0.2 {
            continue;
        }
        if best.as_ref().is_none_or(|(_, _, d)| dot > *d) {
            best = Some((entity, interactable, dot));
        }
    }
    best.map(|(e, i, _)| (e, i))
}
