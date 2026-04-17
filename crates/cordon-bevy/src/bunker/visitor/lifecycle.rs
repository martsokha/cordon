//! Visitor state-machine transitions: arrive → knock → admit →
//! dialogue → dismiss.

use bevy::color::Srgba;
use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use super::audio::{ALARM_VOLUME, AlarmSound, DOOR_VOLUME, DoorSfx};
use super::state::{AdmitVisitor, Visitor, VisitorQueue, VisitorState};
use crate::bunker::camera::FpsCamera;
use crate::bunker::resources::{
    ANTECHAMBER_VISITOR_POS, CameraMode, CurrentDialogue, InteractionLocked, MovementLocked,
    StartDialogue,
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
    ));

    commands.spawn((
        AlarmSound,
        AudioPlayer(door_sfx.alarm.clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(ALARM_VOLUME)),
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
    camera_q: Query<&Transform, With<FpsCamera>>,
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
        ))
        .id();

    crate::bunker::particles::swirl::attach_visitor_arrival_swirl(
        &mut commands,
        &event_assets,
        sprite_entity,
    );

    if let Ok(cam_t) = camera_q.single() {
        *camera_mode = CameraMode::LookingAt {
            target: VISITOR_SPRITE_POS,
            saved_transform: *cam_t,
        };
    }

    start_dialogue.write(StartDialogue {
        node: visitor.yarn_node.clone(),
    });

    for entity in &alarm_q {
        commands.entity(entity).despawn();
    }
    commands.insert_resource(InteractionLocked);
    commands.insert_resource(MovementLocked);
    commands.spawn((
        AudioPlayer(door_sfx.open.clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(DOOR_VOLUME)),
    ));
    info!("visitor admitted: {}", visitor.display_name);
    *state = VisitorState::Inside {
        visitor,
        sprite: sprite_entity,
    };
}

/// When dialogue transitions from active back to Idle and we're
/// still in `Inside`, despawn the sprite, turn the camera back,
/// and return to `Quiet`.
pub(super) fn dismiss_on_dialogue_complete(
    mut commands: Commands,
    mut state: ResMut<VisitorState>,
    mut camera_mode: ResMut<CameraMode>,
    current: Res<CurrentDialogue>,
    mut was_active: Local<bool>,
    door_sfx: Res<DoorSfx>,
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
        if let CameraMode::LookingAt {
            saved_transform, ..
        } = *camera_mode
        {
            *camera_mode = CameraMode::Returning(saved_transform);
        }
        *state = VisitorState::Quiet;
        commands.remove_resource::<InteractionLocked>();
        commands.remove_resource::<MovementLocked>();
        commands.spawn((
            AudioPlayer(door_sfx.close.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(DOOR_VOLUME)),
        ));
        info!("visitor dismissed: {name}");
    }
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
