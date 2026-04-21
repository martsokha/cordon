//! Bunker ambient particles: dust motes, kettle steam, and
//! visitor arrival swirls.

mod dust;
pub mod steam;
pub mod swirl;

use bevy::prelude::*;
use bevy_hanabi::prelude::*;

use crate::PlayingState;

/// Marks a transient emitter entity with the real time at which
/// it should be despawned.
#[derive(Component)]
pub(super) struct EmitterTtl {
    pub despawn_at: f32,
}

/// Shared [`EffectAsset`] handles for per-event emitters.
#[derive(Resource, Clone)]
pub struct EventEffectAssets {
    pub(super) visitor_swirl: Handle<EffectAsset>,
}

pub struct BunkerParticlesPlugin;

impl Plugin for BunkerParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            dust::spawn_dust_emitters.run_if(not(resource_exists::<dust::BunkerDustSpawned>)),
        );
        app.add_systems(
            Startup,
            init_event_effect_assets.run_if(not(resource_exists::<EventEffectAssets>)),
        );
        app.add_systems(
            Update,
            despawn_expired_emitters.run_if(in_state(PlayingState::Bunker)),
        );
    }
}

fn init_event_effect_assets(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    commands.insert_resource(EventEffectAssets {
        visitor_swirl: effects.add(swirl::build_visitor_swirl_effect()),
    });
}

fn despawn_expired_emitters(
    mut commands: Commands,
    time: Res<Time<Real>>,
    q: Query<(Entity, &EmitterTtl)>,
) {
    let now = time.elapsed_secs();
    for (entity, ttl) in &q {
        if now >= ttl.despawn_at {
            commands.entity(entity).despawn();
        }
    }
}
