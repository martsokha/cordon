//! Pill interaction systems: detect MedicationCluster1 props,
//! attach interactables, play a random pill-rattle sound on use.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_sim::resources::{GameClock, PlayerPills};
use rand::RngExt;

use super::components::PillsInteractable;
use super::resources::PillsSfx;
use crate::bunker::geometry::{Prop, PropPlacement};
use crate::bunker::interaction::{Interact, Interactable};

const PILL_VOLUME: f32 = 0.6;
const PILL_VARIANTS: usize = 3;

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
                key: "interact-pills".into(),
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
    clock: Res<GameClock>,
    mut pills: ResMut<PlayerPills>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    // Belt-and-braces: `sync_daily_availability` disables the
    // interactable after the first dose each day, but the observer
    // can still fire if the state machines line up wrong. Re-check
    // here so the dose never double-stamps or the sfx double-plays.
    if pills.last_taken == Some(clock.0.day) {
        return;
    }
    if sfx.clips.is_empty() {
        return;
    }
    let idx = rng.random_range(0..sfx.clips.len());
    commands.spawn((
        AudioPlayer(sfx.clips[idx].clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(PILL_VOLUME)),
    ));
    pills.record_dose(clock.0.day);
    info!("player took pills");
}

/// Once-per-day gate on the pill interactable. Disables the
/// `Interactable` (which hides the "Use pills" hint and makes the
/// prop unclickable) once `PlayerPills.last_taken` matches today.
/// Re-enables it on day rollover automatically.
pub(super) fn sync_daily_availability(
    pills: Res<PlayerPills>,
    clock: Res<GameClock>,
    mut pills_q: Query<&mut Interactable, With<PillsInteractable>>,
) {
    let taken_today = pills.last_taken == Some(clock.0.day);
    for mut interactable in &mut pills_q {
        let should_enable = !taken_today;
        if interactable.enabled != should_enable {
            interactable.enabled = should_enable;
        }
    }
}
