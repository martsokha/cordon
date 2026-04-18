//! CCTV monitor audio: open/close static burst when entering
//! and leaving the fullscreen camera view.

use bevy::prelude::*;

use crate::bunker::resources::CameraMode;

const CCTV_VOLUME: f32 = 0.5;

#[derive(Resource)]
pub(super) struct CctvSfx {
    open: Handle<AudioSource>,
    close: Handle<AudioSource>,
}

pub(super) fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CctvSfx {
        open: asset_server.load("audio/sfx/cctv/open.ogg"),
        close: asset_server.load("audio/sfx/cctv/close.ogg"),
    });
}

/// Play open/close sounds on CameraMode transitions.
pub(super) fn play_on_mode_change(
    mut commands: Commands,
    mode: Res<CameraMode>,
    sfx: Res<CctvSfx>,
    mut was_cctv: Local<bool>,
) {
    let is_cctv = matches!(*mode, CameraMode::AtCctv { .. });
    if !mode.is_changed() {
        return;
    }
    if is_cctv && !*was_cctv {
        commands.spawn((
            AudioPlayer(sfx.open.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(CCTV_VOLUME)),
        ));
    } else if !is_cctv && *was_cctv {
        commands.spawn((
            AudioPlayer(sfx.close.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(CCTV_VOLUME)),
        ));
    }
    *was_cctv = is_cctv;
}
