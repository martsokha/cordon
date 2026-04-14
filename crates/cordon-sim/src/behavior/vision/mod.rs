//! Vision and line-of-sight primitives.
//!
//! Passive data only — no systems. Consumers (engagement scanner,
//! combat) read [`Vision`] radii and compute blocking against
//! [`AnomalyZone`] disks directly. Kept as its own subplugin so the
//! components have a clear home and future LOS-related systems have
//! a place to land without touching combat or squad internals.

pub mod components;

use bevy::prelude::*;

pub use components::{AnomalyZone, Vision};

pub struct VisionPlugin;

impl Plugin for VisionPlugin {
    fn build(&self, _app: &mut App) {
        // No systems today — this subplugin exists to group the
        // vision components and give future LOS work a home.
    }
}
