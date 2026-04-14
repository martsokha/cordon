//! Per-NPC and world-geometry components for vision / line-of-sight.

use bevy::prelude::*;
use cordon_core::primitive::Rank;

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 120 base + 15 per rank tier above Novice.
    pub fn for_npc(rank: Rank) -> Self {
        let radius = 120.0 + (rank.tier() as f32 - 1.0) * 15.0;
        Self { radius }
    }
}

/// Marker for anomaly entities, contributing to LOS blocking. Spawned
/// by the visual layer when it lays out the map; the combat system
/// reads `(Transform, AnomalyZone)` to compute LOS.
#[derive(Component, Debug, Clone, Copy)]
pub struct AnomalyZone {
    pub radius: f32,
}
