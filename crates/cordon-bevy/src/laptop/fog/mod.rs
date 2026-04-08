//! Fog of war for the laptop map.
//!
//! The player "owns" a small set of squads (randomly picked at
//! sim-startup for now — eventually these will be the squads the
//! player recruits). Everything on the map is hidden except within
//! the vision radius of a player-owned squad member.
//!
//! - **NPC dots and relics** reveal in real time. Step out of a
//!   player squad's vision → instantly dark again.
//! - **Areas** are persistent: the first time a player squad lays
//!   eyes on one, its marker stays on the map forever. The fog
//!   overlay on top handles the "currently lit vs memorized"
//!   distinction visually.
//! - The **bunker** dot is always visible.
//! - Player squad members themselves are always visible.
//!
//! This module is split into three implementation files:
//!
//! - [`apply`] — the per-frame visibility loop that hides/reveals
//!   areas, NPCs, relics, and anomaly visuals based on live squad
//!   line-of-sight.
//! - [`trail`] — sampler that drops breadcrumbs at squad
//!   centroids to build a persistent "memory path".
//! - [`sync`] — packs the reveal and memory data into the fog
//!   shader's uniform arrays each frame.

mod apply;
mod sync;
mod trail;

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_sim::components::{FactionId, SquadMarker};

/// Squads the player commands. Membership is set once by
/// [`pick_player_squads`] and never changes afterward (for now).
/// Every fog-related system filters through this set.
#[derive(Resource, Default, Debug)]
pub struct PlayerSquads(pub HashSet<Entity>);

/// Areas that have ever been in sight of a player squad. Once an
/// area enters this set it stays forever — scouting intel doesn't
/// decay. NPCs and relics inside that area still hide/show in real
/// time; only the area disk itself is persistent.
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
/// [`apply::apply_fog`]. Stored in a resource so [`sync`] can
/// write them straight into the shader uniform without
/// recomputing the union each frame.
#[derive(Resource, Default, Debug)]
pub struct FogReveals(pub Vec<(Vec2, f32)>);

/// Cache of the per-frame discovered area disks computed by
/// [`apply::apply_fog`]. Same shape and lifecycle as
/// [`FogReveals`]; `(centre, radius)` per entry.
#[derive(Resource, Default, Debug)]
pub struct DiscoveredDisks(pub Vec<(Vec2, f32)>);

/// Persistent breadcrumb trail of where the player's squads have
/// been. Sampled by [`trail::sample_memory_trail`] and drawn by
/// the fog shader so the path the squad walked stays visible even
/// after they've moved on.
///
/// Capped so the discovered uniform array fits comfortably
/// alongside area disks; oldest entries are evicted first.
#[derive(Resource, Default, Debug)]
pub struct MemoryTrail {
    /// `(centre, radius)` per breadcrumb. Stored in insertion
    /// order; the front is the oldest entry.
    pub points: std::collections::VecDeque<(Vec2, f32)>,
    /// Time-since-startup of the last sample, used to throttle.
    pub last_sample: f32,
}

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
        app.init_resource::<DiscoveredDisks>();
        app.init_resource::<MemoryTrail>();
        app.add_systems(
            Update,
            (
                pick_player_squads,
                apply::apply_fog,
                trail::sample_memory_trail,
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
        .filter(|(_, f)| f.0.as_str() == "drifters")
        .map(|(e, _)| e)
        .collect();
    if candidates.is_empty() {
        return;
    }

    // Deterministic pick: sort by entity bits then stride. Real
    // randomness isn't needed here and bringing in a dep just for
    // this would be overkill — a different set every run would
    // also ruin reproducibility.
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
