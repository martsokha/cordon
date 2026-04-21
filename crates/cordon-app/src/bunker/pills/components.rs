//! Per-entity markers for the pills interaction.

use bevy::prelude::*;

/// Marker so we only attach the interactable once.
#[derive(Component)]
pub(super) struct PillsInteractable;
