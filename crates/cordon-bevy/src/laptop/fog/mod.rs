//! Fog of war for the laptop map.
//!
//! The player "owns" a small set of squads (randomly picked at
//! sim-startup for now — eventually these will be the squads the
//! player recruits). Everything on the map is hidden except
//! within the vision radius of a player-owned squad member.
//!
//! - **NPC dots and relics** reveal in real time. Step out of a
//!   player squad's vision → instantly dark again.
//! - **Areas** are persistent: the first time a player squad
//!   lays eyes on one, its marker stays on the map forever.
//! - **Terrain memory** is tracked per-texel in a 256×256
//!   [`mask::ScoutMask`] texture covering the whole map. Squads
//!   stamp their vision circles into the mask every frame, so
//!   memorised ground grows monotonically and is capped only by
//!   texture resolution. The fog shader samples the mask to
//!   decide which regions to paint with the grey "memory wash"
//!   vs. the swirly "never seen" cloud.
//! - The **bunker** dot is always visible.
//! - Player squad members themselves are always visible.
//!
//! This module is split into:
//!
//! - [`apply`] — per-frame visibility loop for areas, NPCs,
//!   relics, and anomaly visuals.
//! - [`mask`] — the persistent scout-mask texture and the
//!   per-frame stamping system.
//! - [`sync`] — packs the reveal-circle uniform into the fog
//!   shader material each frame.

mod apply;
pub(crate) mod mask;
mod sync;

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_sim::components::{FactionId, SquadMarker};

pub use self::mask::ScoutMask;

/// Squads the player commands. Membership is set once by
/// [`pick_player_squads`] and never changes afterward (for now).
/// Every fog-related system filters through this set.
#[derive(Resource, Default, Debug)]
pub struct PlayerSquads(pub HashSet<Entity>);

/// Areas that have ever been in sight of a player squad. Once
/// an area enters this set it stays forever — scouting intel
/// doesn't decay. NPCs and relics inside that area still
/// hide/show in real time; only the area mesh visibility is
/// driven by this latch.
#[derive(Resource, Default, Debug)]
pub struct RevealedAreas(pub HashSet<Entity>);

/// Master fog toggle. When `enabled = false`, everything on the
/// map is visible regardless of player squad line-of-sight — the
/// F3 debug cheat flips this.
#[derive(Resource, Debug)]
pub struct FogEnabled {
    pub enabled: bool,
}

impl Default for FogEnabled {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Cache of the per-frame reveal circles computed by
/// [`apply::apply_fog`]. Stored in a resource so [`sync`] and
/// [`mask::update_scout_mask`] can read them without re-walking
/// the player-squad member list.
#[derive(Resource, Default, Debug)]
pub struct FogReveals(pub Vec<(Vec2, f32)>);

/// Number of squads the player starts owning. Pulled from the
/// drifter faction so there's always something to pick from —
/// drifters are the neutral, always-present faction.
const PLAYER_SQUAD_COUNT: usize = 3;

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSquads>();
        app.init_resource::<RevealedAreas>();
        app.init_resource::<FogEnabled>();
        app.init_resource::<FogReveals>();
        app.add_plugins(mask::ScoutMaskPlugin);
        app.add_systems(
            Update,
            (
                pick_player_squads,
                apply::apply_fog,
                mask::update_scout_mask,
                sync::sync_fog_material,
            )
                .chain()
                .after(cordon_sim::plugin::SimSet::Spawn)
                .run_if(in_state(crate::AppState::Playing)),
        );
    }
}

/// Pick a few drifter squads to be the player's once the sim has
/// finished spawning. Idempotent — bails if the set is already
/// non-empty.
fn pick_player_squads(
    mut player_squads: ResMut<PlayerSquads>,
    squads: Query<(Entity, &FactionId), With<SquadMarker>>,
) {
    if !player_squads.0.is_empty() {
        return;
    }

    // Collect drifter squads first; if there are none yet, bail
    // and try again next frame. The sim sometimes takes a couple
    // of frames to finish spawning.
    let mut candidates: Vec<Entity> = squads
        .iter()
        .filter(|(_, f)| f.0.as_str() == "faction_drifters")
        .map(|(e, _)| e)
        .collect();
    if candidates.is_empty() {
        return;
    }

    // Deterministic pick: sort by entity bits then stride. Real
    // randomness isn't needed here and bringing in a dep just
    // for this would be overkill — a different set every run
    // would also ruin reproducibility.
    candidates.sort_by_key(|e| e.to_bits());
    let step = (candidates.len() / PLAYER_SQUAD_COUNT.max(1)).max(1);
    for (i, entity) in candidates.into_iter().step_by(step).enumerate() {
        if i >= PLAYER_SQUAD_COUNT {
            break;
        }
        player_squads.0.insert(entity);
    }

    info!(
        "fog: picked {} player squads from drifters",
        player_squads.0.len()
    );
}
