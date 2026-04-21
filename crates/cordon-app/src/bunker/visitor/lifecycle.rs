//! Visitor state-machine transitions: arrive → knock → admit →
//! dialogue → dismiss.

use bevy::audio::Volume;
use bevy::color::Srgba;
use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::entity::npc::TemplateId;
use cordon_sim::quest::messages::DismissTemplateNpc;

use super::audio::{ALARM_VOLUME, AlarmSound, DOOR_CLOSE_VOLUME, DOOR_OPEN_VOLUME, DoorSfx};
use super::state::{AdmitVisitor, PendingStepAway, Visitor, VisitorQueue, VisitorState};
use crate::bunker::interaction::{Interact, Interactable, InteractableWhileCarrying};
use crate::bunker::resources::{
    ANTECHAMBER_VISITOR_POS, CameraMode, CurrentDialogue, DialogueOwner, InteractionLocked,
    MovementLocked, StartDialogue,
};

/// Marker for the in-bunker visitor sprite (the one shown across
/// the desk during dialogue). Despawned when the dialogue ends.
#[derive(Component)]
struct VisitorSprite;

/// Marker for the antechamber preview sprite (the one the CCTV
/// camera films while a visitor is knocking). Despawned when
/// state leaves `Knocking`.
#[derive(Component)]
pub(super) struct KnockingPreview;

/// World-space position of the placeholder visitor sprite — also
/// the point the camera turns to face during dialogue.
const VISITOR_SPRITE_POS: Vec3 = Vec3::new(0.0, 1.2, 2.4);

/// When the door is quiet and the queue is non-empty, pop the next
/// visitor, transition to Knocking, and spawn a preview sprite in
/// the hidden antechamber so the CCTV camera has something to film.
pub(super) fn arrive_next_visitor(
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut queue: ResMut<VisitorQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    door_sfx: Res<DoorSfx>,
    game_data: Res<GameDataResource>,
) {
    if !matches!(*state, VisitorState::Quiet) {
        return;
    }
    let Some(visitor) = queue.0.pop_front() else {
        return;
    };

    let sprite_color = faction_sprite_color(&game_data, &visitor);
    commands.spawn((
        KnockingPreview,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.4, 0.9)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: sprite_color,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(ANTECHAMBER_VISITOR_POS)
            .looking_at(ANTECHAMBER_VISITOR_POS + Vec3::new(0.0, 0.0, 1.0), Vec3::Y),
        DespawnOnExit(crate::AppState::Playing),
    ));

    commands.spawn((
        AlarmSound,
        AudioPlayer(door_sfx.alarm.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(ALARM_VOLUME)),
        DespawnOnExit(crate::AppState::Playing),
    ));
    info!("visitor arrived: {}", visitor.display_name);
    *state = VisitorState::Knocking { visitor };
}

/// Despawn the antechamber preview sprite once the visitor has been
/// admitted (or otherwise left the queue).
pub(super) fn despawn_preview_on_leave_knocking(
    mut commands: Commands,
    state: Res<VisitorState>,
    preview_q: Query<Entity, With<KnockingPreview>>,
) {
    if !state.is_changed() {
        return;
    }
    if matches!(*state, VisitorState::Knocking { .. }) {
        return;
    }
    for entity in &preview_q {
        commands.entity(entity).despawn();
    }
}

/// Handle [`AdmitVisitor`] messages: spawn the visitor sprite, turn
/// the camera, and ask the dialogue module to start the yarn node.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_admit_visitor(
    mut commands: Commands,
    mut requests: MessageReader<AdmitVisitor>,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    mut start_dialogue: MessageWriter<StartDialogue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    event_assets: Res<crate::bunker::particles::EventEffectAssets>,
    door_sfx: Res<DoorSfx>,
    alarm_q: Query<Entity, With<AlarmSound>>,
    game_data: Res<GameDataResource>,
) {
    if requests.read().next().is_none() {
        return;
    }
    let visitor = match &*state {
        VisitorState::Knocking { visitor } => visitor.clone(),
        _ => return,
    };

    let sprite_color = faction_sprite_color(&game_data, &visitor);
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
            DespawnOnExit(crate::AppState::Playing),
        ))
        .id();

    crate::bunker::particles::swirl::attach_visitor_arrival_swirl(
        &mut commands,
        &event_assets,
        sprite_entity,
    );

    *camera_mode = CameraMode::LookingAt {
        target: VISITOR_SPRITE_POS,
    };

    start_dialogue.write(StartDialogue {
        node: visitor.yarn_node.clone(),
        by: DialogueOwner::Visitor,
    });

    for entity in &alarm_q {
        commands.entity(entity).despawn();
    }
    commands.insert_resource(InteractionLocked);
    commands.insert_resource(MovementLocked);
    commands.spawn((
        AudioPlayer(door_sfx.open.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(DOOR_OPEN_VOLUME)),
    ));
    info!("visitor admitted: {}", visitor.display_name);
    *state = VisitorState::Inside {
        visitor,
        sprite: sprite_entity,
    };
}

/// On dialogue end, decide: dismiss the visitor, or transition
/// to `Waiting` so the player can step away and come back.
///
/// If the conversation called `<<step_away "node">>`, the
/// visitor stays around as an interactable sprite with the
/// given node as the resume target. Otherwise the visitor
/// leaves normally — covering "dialogue ran out of lines" and
/// "yarn author just let the node end", keeping the common
/// case trivial.
pub(super) fn dismiss_on_dialogue_complete(
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    current: Res<CurrentDialogue>,
    step_away: Option<Res<PendingStepAway>>,
    mut was_active: Local<bool>,
    door_sfx: Res<DoorSfx>,
    mut dismiss_sim_npc: MessageWriter<DismissTemplateNpc>,
    template_q: Query<(Entity, &TemplateId)>,
) {
    let now_active = !matches!(*current, CurrentDialogue::Idle);
    let just_ended = *was_active && !now_active;
    *was_active = now_active;

    if !just_ended {
        return;
    }
    let VisitorState::Inside { visitor, sprite } = &*state else {
        // Dialogue ended without an `Inside` visitor (e.g. a
        // narrator-only quest cutscene). Clear any stray flag
        // so it doesn't leak into the next real conversation.
        commands.remove_resource::<PendingStepAway>();
        return;
    };

    let resume_node = step_away.map(|s| s.resume_node.clone());
    commands.remove_resource::<PendingStepAway>();

    // Release the camera to free look without snapping back to
    // the admit-time transform. The player already sees the
    // visitor on-screen; yanking them back to where they were
    // standing before E'ing the button is jarring, especially
    // on the step-away path where the natural next action is
    // "turn and walk to the rack". Both step-away and dismiss
    // take this path.
    if matches!(*camera_mode, CameraMode::LookingAt { .. }) {
        *camera_mode = CameraMode::Free;
    }

    if let Some(resume_node) = resume_node {
        // Transition to Waiting: sprite lives on, player gets
        // control back.
        let visitor = visitor.clone();
        let sprite_entity = *sprite;
        commands.remove_resource::<InteractionLocked>();
        commands.remove_resource::<MovementLocked>();
        info!(
            "visitor waiting: {} (resume at `{resume_node}`)",
            visitor.display_name
        );
        *state = VisitorState::Waiting {
            visitor,
            sprite: sprite_entity,
            resume_node,
        };
        return;
    }

    // Default path: dismiss. Despawn the sprite and send the
    // sim-side NPC home. The two are intentionally paired so
    // the bunker visitor and the map NPC always appear together
    // and leave together — quest advance no longer dismisses
    // the sim side prematurely.
    let name = visitor.display_name.clone();
    let sprite_entity = *sprite;
    if let Some(template_id) = visitor.template.clone() {
        let sim_entity = template_q
            .iter()
            .find_map(|(entity, tid)| (tid.0 == template_id).then_some(entity));
        if let Some(sim_entity) = sim_entity {
            dismiss_sim_npc.write(DismissTemplateNpc {
                entity: sim_entity,
                template: template_id.clone(),
            });
            info!(
                "visitor dismissed: {name} (sim template `{}` heading home)",
                template_id.as_str()
            );
        } else {
            warn!(
                "visitor dismissed: {name} but no live sim entity for template `{}`",
                template_id.as_str()
            );
        }
    } else {
        info!("visitor dismissed: {name}");
    }
    commands.entity(sprite_entity).despawn();
    *state = VisitorState::Quiet;
    commands.remove_resource::<InteractionLocked>();
    commands.remove_resource::<MovementLocked>();
    commands.spawn((
        AudioPlayer(door_sfx.close.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(DOOR_CLOSE_VOLUME)),
    ));
}

/// When entering `Waiting`, attach an [`Interactable`] to the
/// visitor sprite so the player can press E to resume dialogue.
/// When leaving `Waiting`, strip the component. Change-gated so
/// it's free at steady state.
pub(super) fn update_waiting_interactable(mut commands: Commands, state: Res<VisitorState>) {
    if !state.is_changed() {
        return;
    }
    match &*state {
        VisitorState::Waiting { sprite, .. } => {
            commands.entity(*sprite).insert((
                Interactable {
                    key: "interact-visitor".into(),
                    enabled: true,
                },
                // Opt out of `block_non_rack_interactions` — the
                // whole point of the step-away loop is that the
                // player comes back carrying something.
                InteractableWhileCarrying,
            ));
        }
        VisitorState::Inside { sprite, .. } => {
            // Inside re-locks interactions, but strip the
            // interactable anyway so the prompt never shows up
            // on top of the dialogue UI.
            commands
                .entity(*sprite)
                .remove::<Interactable>()
                .remove::<InteractableWhileCarrying>();
        }
        VisitorState::Quiet | VisitorState::Knocking { .. } => {}
    }
}

/// Observer: when the player interacts with a visitor sprite
/// in `Waiting`, resume dialogue at the carried `resume_node`
/// and transition back to `Inside`. The observer is attached
/// per-sprite so only that sprite's interaction fires it.
pub(super) fn on_visitor_interact(
    _trigger: On<Interact>,
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    mut start_dialogue: MessageWriter<StartDialogue>,
) {
    let VisitorState::Waiting {
        visitor,
        sprite,
        resume_node,
    } = &*state
    else {
        return;
    };
    let visitor = visitor.clone();
    let sprite_entity = *sprite;
    let resume_node = resume_node.clone();
    // Strip both markers on the same frame as the transition.
    // `update_waiting_interactable` would catch the `Inside`
    // branch next frame, but leaving `InteractableWhileCarrying`
    // on a non-interactable sprite between frames is a minor
    // incoherence worth avoiding.
    commands
        .entity(sprite_entity)
        .remove::<Interactable>()
        .remove::<InteractableWhileCarrying>();
    // Re-lock the camera on the visitor.
    *camera_mode = CameraMode::LookingAt {
        target: VISITOR_SPRITE_POS,
    };
    commands.insert_resource(InteractionLocked);
    commands.insert_resource(MovementLocked);
    info!(
        "visitor resumed: {} at node `{resume_node}`",
        visitor.display_name
    );
    start_dialogue.write(StartDialogue {
        node: resume_node,
        by: DialogueOwner::Visitor,
    });
    *state = VisitorState::Inside {
        visitor,
        sprite: sprite_entity,
    };
}

/// Reset visitor lifecycle state on entering a fresh run.
///
/// [`DespawnOnExit(AppState::Playing)`] already cleans up the
/// sprite entity on exit, but `VisitorState` is a long-lived
/// resource that would otherwise carry over a stale `Waiting
/// { sprite: <despawned> }` into the next run. Also clears the
/// visitor queue and drops the pending step-away flag so
/// nothing bleeds across runs.
pub fn reset_visitor_state(
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut queue: ResMut<VisitorQueue>,
) {
    *state = VisitorState::Quiet;
    queue.0.clear();
    commands.remove_resource::<PendingStepAway>();
}

fn faction_sprite_color(game_data: &GameDataResource, visitor: &Visitor) -> Color {
    game_data
        .0
        .factions
        .get(&visitor.faction)
        .and_then(|def| Srgba::hex(&def.color).ok())
        .map(Color::Srgba)
        .unwrap_or(Color::srgb(0.6, 0.6, 0.6))
}
