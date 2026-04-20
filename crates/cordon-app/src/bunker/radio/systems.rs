use bevy::audio::Volume;
use bevy::prelude::*;
use cordon_sim::day::radio::RadioBroadcast;

use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::RadioPlacement;

const STATIC_VOLUME: f32 = 0.16;
const CHATTER_VOLUME: f32 = 0.35;
const TOGGLE_VOLUME: f32 = 0.75;

#[derive(Resource)]
pub(super) struct RadioSfx {
    static_loop: Handle<AudioSource>,
    chatter: Handle<AudioSource>,
    enable: Handle<AudioSource>,
    disable: Handle<AudioSource>,
}

/// Marker on the radio entity.
#[derive(Component)]
pub(crate) struct RadioMarker;

/// Whether the radio is currently on.
#[derive(Component)]
pub(crate) struct RadioOn(bool);

impl RadioOn {
    pub(crate) fn is_on(&self) -> bool {
        self.0
    }
}

/// Marker on any audio entity owned by the radio. All of these
/// are killed on toggle-off so nothing keeps playing.
#[derive(Component)]
struct RadioAudio;

pub(super) fn load_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RadioSfx {
        static_loop: asset_server.load("audio/sfx/radio/static_01.ogg"),
        chatter: asset_server.load("audio/sfx/radio/chatter.ogg"),
        enable: asset_server.load("audio/sfx/radio/enable.ogg"),
        disable: asset_server.load("audio/sfx/radio/disable.ogg"),
    });
}

pub(super) fn spawn_radio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sfx: Res<RadioSfx>,
    placement: Option<Res<RadioPlacement>>,
) {
    let Some(placement) = placement else { return };
    let scene: Handle<Scene> = asset_server.load("models/lowpoly/Radio_02.glb#Scene0");

    let radio = commands
        .spawn((
            RadioMarker,
            RadioOn(true),
            Interactable {
                key: "interact-radio-off".into(),
                enabled: true,
            },
            SceneRoot(scene),
            Transform::from_translation(placement.pos).with_rotation(placement.rot),
        ))
        .observe(on_toggle)
        .id();

    spawn_static_audio(&mut commands, radio, &sfx);
    commands.remove_resource::<RadioPlacement>();
}

fn spawn_static_audio(commands: &mut Commands, parent: Entity, sfx: &RadioSfx) {
    let audio = commands
        .spawn((
            RadioAudio,
            AudioPlayer(sfx.static_loop.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: Volume::Linear(STATIC_VOLUME),
                spatial: true,
                ..default()
            },
            // Position inherited from parent — Transform::default()
            // so the audio sits at the radio prop, not offset.
            Transform::default(),
        ))
        .id();
    commands.entity(parent).add_child(audio);
}

fn on_toggle(
    _trigger: On<Interact>,
    mut commands: Commands,
    sfx: Res<RadioSfx>,
    mut radio_q: Query<
        (Entity, &mut RadioOn, &mut Interactable, &GlobalTransform),
        With<RadioMarker>,
    >,
    audio_q: Query<Entity, With<RadioAudio>>,
) {
    let Ok((radio_entity, mut on, mut interactable, transform)) = radio_q.single_mut() else {
        return;
    };

    on.0 = !on.0;

    let pos = transform.translation();
    if on.0 {
        interactable.key = "interact-radio-off".into();
        spawn_oneshot(&mut commands, &sfx.enable, TOGGLE_VOLUME, pos, true);
        spawn_static_audio(&mut commands, radio_entity, &sfx);
    } else {
        interactable.key = "interact-radio-on".into();
        // Kill all radio audio (static, chatter, enable sound).
        for entity in &audio_q {
            commands.entity(entity).despawn();
        }
        // Disable sound plays without RadioAudio so it survives.
        spawn_oneshot(&mut commands, &sfx.disable, TOGGLE_VOLUME, pos, false);
    }
}

fn spawn_oneshot(
    commands: &mut Commands,
    handle: &Handle<AudioSource>,
    volume: f32,
    pos: Vec3,
    tracked: bool,
) {
    let mut entity = commands.spawn((
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            spatial: true,
            ..default()
        },
        Transform::from_translation(pos),
    ));
    if tracked {
        entity.insert(RadioAudio);
    }
}

/// Play a short chatter sting whenever a broadcast arrives and the
/// radio is on. Audio feedback that "something new is queued";
/// the actual broadcast content plays through the dialogue UI when
/// the player tunes in.
///
/// [`BroadcastHeard`] is written by the queue module, not here.
pub(super) fn play_broadcast(
    mut commands: Commands,
    sfx: Res<RadioSfx>,
    radio_q: Query<(&RadioOn, &GlobalTransform), With<RadioMarker>>,
    mut broadcasts: MessageReader<RadioBroadcast>,
) {
    let Ok((on, transform)) = radio_q.single() else {
        broadcasts.read().for_each(drop);
        return;
    };

    let mut played = false;
    for _msg in broadcasts.read() {
        if !on.0 {
            continue;
        }
        played = true;
    }

    if played {
        spawn_oneshot(
            &mut commands,
            &sfx.chatter,
            CHATTER_VOLUME,
            transform.translation(),
            true,
        );
    }
}
