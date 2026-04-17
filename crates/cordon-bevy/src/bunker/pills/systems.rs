//! Pill interaction systems: detect MedicationCluster1 props,
//! attach interactables, play a random pill-rattle sound on use.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use rand::RngExt;

use crate::bunker::geometry::{Prop, PropPlacement};
use crate::bunker::interaction::{Interact, Interactable};

const PILL_VOLUME: f32 = 0.6;
const PILL_VARIANTS: usize = 3;

/// Marker so we only attach the interactable once.
#[derive(Component)]
pub(super) struct PillsInteractable;

#[derive(Resource)]
struct PillsSfx {
    clips: Vec<Handle<AudioSource>>,
}

pub(super) fn load_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    let clips: Vec<_> = (1..=PILL_VARIANTS)
        .map(|i| asset_server.load(format!("audio/sfx/pills/take_{i:02}.ogg")))
        .collect();
    commands.insert_resource(PillsSfx { clips });
}

/// Detect MedicationCluster1 props that have been resolved
/// (have SceneRoot) and attach an Interactable to them.
pub(super) fn attach_interactable(
    mut commands: Commands,
    clusters: Query<(Entity, &PropPlacement), (With<SceneRoot>, Without<PillsInteractable>)>,
) {
    for (entity, placement) in &clusters {
        if placement.kind != Prop::MedicationCluster1 {
            continue;
        }
        commands.entity(entity).insert((
            PillsInteractable,
            Interactable {
                prompt: "[E] Take pills".into(),
                enabled: true,
            },
        ));
    }
}

/// Attach the interaction observer to newly-tagged pill entities.
pub(super) fn attach_observer(
    mut commands: Commands,
    new: Query<Entity, Added<PillsInteractable>>,
) {
    for entity in &new {
        commands.entity(entity).observe(on_take_pills);
    }
}

fn on_take_pills(
    _trigger: On<Interact>,
    mut commands: Commands,
    sfx: Res<PillsSfx>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    if sfx.clips.is_empty() {
        return;
    }
    let idx = rng.random_range(0..sfx.clips.len());
    commands.spawn((
        AudioPlayer(sfx.clips[idx].clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(PILL_VOLUME)),
    ));
    info!("player took pills");
}
