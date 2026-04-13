use bevy::prelude::*;

#[derive(Component)]
pub struct FpsCamera;

#[derive(Component)]
pub struct LaptopObject;

#[derive(Component)]
pub struct InteractPrompt;

/// Marker for the small dome on the desk that admits a knocking
/// visitor. Spawned by the room module; clicked via mesh picking.
#[derive(Component)]
pub struct DoorButton;
