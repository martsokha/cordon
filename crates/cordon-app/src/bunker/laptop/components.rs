use bevy::prelude::*;

/// Marker for the laptop mesh in the bunker. Used by the interaction
/// system to identify the laptop as an interactable target.
#[derive(Component)]
pub struct LaptopObject;
