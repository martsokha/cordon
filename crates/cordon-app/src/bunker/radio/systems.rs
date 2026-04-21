//! Radio prop spawn, audio handles, and the listening-mode
//! interact handler.
//!
//! Interacting with the radio no longer toggles an on/off state —
//! it enters a focused listening mode where the player reads
//! queued broadcasts one by one through the dialogue UI. Exit is
//! handled by [`queue`](super::queue).

use bevy::audio::Volume;
use bevy::prelude::*;

use super::queue::EnterListening;
use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::RadioPlacement;
use crate::bunker::visitor::VisitorState;

pub(super) const STATIC_VOLUME: f32 = 0.35;
pub(super) const STATIC_BURST_VOLUME: f32 = 0.45;
pub(super) const CHATTER_VOLUME: f32 = 0.4;
pub(super) const TOGGLE_VOLUME: f32 = 0.75;

#[derive(Resource)]
pub(super) struct RadioSfx {
    pub(super) static_loop: Handle<AudioSource>,
    pub(super) static_burst: Handle<AudioSource>,
    pub(super) chatter: Handle<AudioSource>,
    pub(super) enable: Handle<AudioSource>,
    pub(super) disable: Handle<AudioSource>,
}

/// Marker on the radio prop entity.
#[derive(Component)]
pub(crate) struct RadioMarker;

/// Marker on any audio entity owned by the radio. All of these
/// are despawned when listening ends so nothing keeps playing.
#[derive(Component)]
pub(super) struct RadioAudio;

/// Subset marker for the chatter loop that accompanies a
/// currently-reading broadcast. Despawned when the broadcast
/// dialog ends — independent of the ambient static loop.
#[derive(Component)]
pub(super) struct RadioChatter;

pub(super) fn load_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RadioSfx {
        static_loop: asset_server.load("audio/sfx/radio/static_01.ogg"),
        static_burst: asset_server.load("audio/sfx/radio/static_02.ogg"),
        chatter: asset_server.load("audio/sfx/radio/chatter.ogg"),
        enable: asset_server.load("audio/sfx/radio/enable.ogg"),
        disable: asset_server.load("audio/sfx/radio/disable.ogg"),
    });
}

pub(super) fn spawn_radio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placement: Option<Res<RadioPlacement>>,
    visitor_state: Res<VisitorState>,
) {
    let Some(placement) = placement else { return };
    let scene: Handle<Scene> = asset_server.load("models/lowpoly/Radio_02.glb#Scene0");

    // Seed `enabled` from the current visitor state — if someone's
    // already at the door when the radio spawns, the prompt starts
    // hidden. `sync_radio_interactable` keeps it in sync thereafter.
    let enabled = matches!(*visitor_state, VisitorState::Quiet);

    commands
        .spawn((
            RadioMarker,
            Interactable {
                key: "interact-radio-listen".into(),
                enabled,
            },
            SceneRoot(scene),
            Transform::from_translation(placement.pos).with_rotation(placement.rot),
        ))
        .observe(on_interact);
    commands.remove_resource::<RadioPlacement>();
}

/// Interact handler: request to enter listening mode. The actual
/// state transition is owned by the queue module so all
/// lock/unlock bookkeeping lives in one place.
///
/// Visitor state is gated by
/// [`sync_radio_interactable`] which flips `Interactable.enabled`
/// so the prompt is hidden entirely while a visitor is around —
/// by the time a click reaches here, listening is already allowed.
fn on_interact(_trigger: On<Interact>, mut enter_tx: MessageWriter<EnterListening>) {
    enter_tx.write(EnterListening);
}

/// Keep the radio's interaction prompt in sync with visitor
/// state: prompt is enabled only when [`VisitorState::Quiet`].
/// Any non-quiet state (knocking, inside, waiting) disables the
/// prop so the hint disappears and the player can't even try to
/// click — the visitor flow owns the player's attention.
///
/// Gated on `VisitorState::is_changed()` so the query/update only
/// fires on state transitions, not every frame.
pub(super) fn sync_radio_interactable(
    visitor_state: Res<VisitorState>,
    mut radio_q: Query<&mut Interactable, With<RadioMarker>>,
) {
    if !visitor_state.is_changed() {
        return;
    }
    let allow = matches!(*visitor_state, VisitorState::Quiet);
    for mut interactable in &mut radio_q {
        if interactable.enabled != allow {
            interactable.enabled = allow;
        }
    }
}

/// Spawn the static-loop audio attached to the radio prop. Used
/// once when listening starts; killed when listening ends.
pub(super) fn spawn_static_loop(commands: &mut Commands, parent: Entity, sfx: &RadioSfx) {
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
            Transform::default(),
        ))
        .id();
    commands.entity(parent).add_child(audio);
}

/// Play a one-shot sound at a world position, tagged as radio
/// audio so it gets cleaned up on exit.
pub(super) fn spawn_oneshot_at(
    commands: &mut Commands,
    handle: &Handle<AudioSource>,
    volume: f32,
    pos: Vec3,
) {
    commands.spawn((
        RadioAudio,
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            spatial: true,
            ..default()
        },
        Transform::from_translation(pos),
    ));
}

/// One-shot click sound (enable / disable). Not tagged with
/// `RadioAudio` so it survives the exit teardown — otherwise the
/// disable click would cut itself off mid-play.
pub(super) fn spawn_click(
    commands: &mut Commands,
    handle: &Handle<AudioSource>,
    volume: f32,
    pos: Vec3,
) {
    commands.spawn((
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            spatial: true,
            ..default()
        },
        Transform::from_translation(pos),
    ));
}

/// Spawn the chatter sting that plays once when a broadcast
/// begins. Carries both `RadioAudio` (cleaned up on exit) and
/// `RadioChatter` (explicitly despawned at broadcast-end so a
/// mid-read exit doesn't leave it drifting over the static loop).
/// `PlaybackMode::Despawn` ensures it's torn down automatically
/// when the clip finishes even if the broadcast dialog is still
/// running — one sting per broadcast.
pub(super) fn spawn_chatter(commands: &mut Commands, parent: Entity, sfx: &RadioSfx) {
    let audio = commands
        .spawn((
            RadioAudio,
            RadioChatter,
            AudioPlayer(sfx.chatter.clone()),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: Volume::Linear(CHATTER_VOLUME),
                spatial: true,
                ..default()
            },
            Transform::default(),
        ))
        .id();
    commands.entity(parent).add_child(audio);
}
