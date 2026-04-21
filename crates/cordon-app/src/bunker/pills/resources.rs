//! Shared resources for the pills interaction.

use bevy::prelude::*;

/// Preloaded pill-rattle sfx clips. One of these is picked at
/// random each time the player takes a dose.
#[derive(Resource)]
pub(super) struct PillsSfx {
    pub clips: Vec<Handle<AudioSource>>,
}
