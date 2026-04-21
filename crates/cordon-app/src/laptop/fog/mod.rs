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

pub use self::mask::ScoutMask;

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

pub struct FogPlugin;

impl Plugin for FogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RevealedAreas>();
        app.init_resource::<FogEnabled>();
        app.init_resource::<FogReveals>();
        app.add_plugins(mask::ScoutMaskPlugin);
        app.add_systems(
            Update,
            (
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
