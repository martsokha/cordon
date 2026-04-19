//! CCTV monitor audio: open/close static burst when entering
//! and leaving the fullscreen camera view.

use bevy::prelude::*;

use crate::bunker::resources::CameraMode;

const CCTV_VOLUME: f32 = 0.5;

/// Marker for the currently-playing open/close sfx entity, so a
/// rapid open→close→open sequence cuts the previous burst
/// instead of stacking overlapping plays.
#[derive(Component)]
pub(super) struct CctvSfxInstance;

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

/// Play open/close sounds on CameraMode transitions. Mutually
/// exclusive — any previous instance is despawned first so a
/// quick toggle doesn't layer the two bursts over each other.
pub(super) fn play_on_mode_change(
    mut commands: Commands,
    mode: Res<CameraMode>,
    sfx: Res<CctvSfx>,
    existing: Query<Entity, With<CctvSfxInstance>>,
    mut was_cctv: Local<bool>,
) {
    let is_cctv = matches!(*mode, CameraMode::AtCctv { .. });
    if !mode.is_changed() {
        return;
    }
    let handle = if is_cctv && !*was_cctv {
        Some(sfx.open.clone())
    } else if !is_cctv && *was_cctv {
        Some(sfx.close.clone())
    } else {
        None
    };
    if let Some(handle) = handle {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        commands.spawn((
            CctvSfxInstance,
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(CCTV_VOLUME)),
        ));
    }
    *was_cctv = is_cctv;
}
