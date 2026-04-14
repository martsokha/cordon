//! Per-NPC movement components.

use bevy::prelude::*;

/// The point this NPC is currently walking toward, in world space.
/// `None` means the NPC is standing still.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct MovementTarget(pub Option<Vec2>);

/// How fast this NPC walks toward [`MovementTarget`], in map units per
/// second. Updated by the system that sets the target.
#[derive(Component, Debug, Clone, Copy)]
pub struct MovementSpeed(pub f32);

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(30.0)
    }
}
