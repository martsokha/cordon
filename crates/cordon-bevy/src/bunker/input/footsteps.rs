//! Footstep audio: reacts to [`FootstepScuffed`] events from the
//! controller and plays a surface-appropriate clip.
//!
//! Surface is inferred from the world: if the player's feet land
//! inside any [`Prop::Rug`] entity's footprint the carpet set
//! plays; otherwise concrete. No hardcoded room bounds.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use rand::RngExt;

use super::controller::FootstepScuffed;
use crate::PlayingState;
use crate::bunker::geometry::{Prop, PropPlacement};

const FOOTSTEP_VOLUME: f32 = 0.45;
const VARIANTS_PER_SURFACE: usize = 4;

#[derive(Clone, Copy)]
enum Surface {
    Concrete,
    Carpet,
}

impl Surface {
    fn prefix(self) -> &'static str {
        match self {
            Self::Concrete => "concrete",
            Self::Carpet => "carpet",
        }
    }

    fn load_clips(self, asset_server: &AssetServer) -> Vec<Handle<AudioSource>> {
        (1..=VARIANTS_PER_SURFACE)
            .map(|i| {
                asset_server.load(format!("audio/sfx/footstep/{}_{:02}.ogg", self.prefix(), i,))
            })
            .collect()
    }
}

#[derive(Resource)]
struct FootstepSfx {
    concrete: Vec<Handle<AudioSource>>,
    carpet: Vec<Handle<AudioSource>>,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, load);
    app.add_systems(Update, play.run_if(in_state(PlayingState::Bunker)));
}

fn load(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(FootstepSfx {
        concrete: Surface::Concrete.load_clips(&asset_server),
        carpet: Surface::Carpet.load_clips(&asset_server),
    });
}

fn play(
    mut commands: Commands,
    mut steps: MessageReader<FootstepScuffed>,
    sfx: Res<FootstepSfx>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    rugs: Query<(&PropPlacement, &Transform)>,
) {
    for ev in steps.read() {
        let surface = if on_rug(ev.pos, &rugs) {
            Surface::Carpet
        } else {
            Surface::Concrete
        };
        let pool = match surface {
            Surface::Carpet => &sfx.carpet,
            Surface::Concrete => &sfx.concrete,
        };
        if pool.is_empty() {
            continue;
        }
        let idx = rng.random_range(0..pool.len());
        commands.spawn((
            AudioPlayer(pool[idx].clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(FOOTSTEP_VOLUME)),
        ));
    }
}

/// Check whether `pos` falls inside any rug's XZ footprint.
/// Uses the prop's AABB + world transform to derive the
/// world-space rectangle — no hardcoded room bounds needed.
fn on_rug(pos: Vec3, rugs: &Query<(&PropPlacement, &Transform)>) -> bool {
    rugs.iter()
        .filter(|(p, _)| p.kind == Prop::Rug)
        .any(|(p, transform)| {
            let def = p.kind.def();
            let half_x = (def.aabb_max.x - def.aabb_min.x) * 0.5 * p.scale;
            let half_z = (def.aabb_max.z - def.aabb_min.z) * 0.5 * p.scale;
            let center = transform.translation;
            (pos.x - center.x).abs() <= half_x && (pos.z - center.z).abs() <= half_z
        })
}
