//! Ambient audio systems.

use bevy::prelude::*;

const AIRFLOW_VOLUME: f32 = 0.15;

/// Flag so the ambient loop only spawns once.
#[derive(Resource)]
pub(super) struct AmbientSpawned;

pub(super) fn start_ambient(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<AudioSource> = asset_server.load("audio/ambient/airflow.ogg");
    commands.spawn((
        AudioPlayer(handle),
        PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(AIRFLOW_VOLUME)),
    ));
    commands.insert_resource(AmbientSpawned);
}
