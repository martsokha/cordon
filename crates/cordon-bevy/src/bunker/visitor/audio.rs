//! Door audio: alarm, open, close — loaded once, played by the
//! lifecycle systems.

use bevy::prelude::*;

/// Preloaded door audio handles.
#[derive(Resource)]
pub(super) struct DoorSfx {
    pub alarm: Handle<AudioSource>,
    pub open: Handle<AudioSource>,
    pub close: Handle<AudioSource>,
}

/// Tag on the alarm audio entity so it can be despawned when the
/// player admits the visitor.
#[derive(Component)]
pub(super) struct AlarmSound;

pub(super) const DOOR_VOLUME: f32 = 0.6;
pub(super) const ALARM_VOLUME: f32 = 0.15;

pub(super) fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(DoorSfx {
        alarm: asset_server.load("audio/sfx/door/alarm.ogg"),
        open: asset_server.load("audio/sfx/door/open.ogg"),
        close: asset_server.load("audio/sfx/door/close.ogg"),
    });
}
