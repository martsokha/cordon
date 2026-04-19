//! Laptop interaction feature: owns the LaptopObject component,
//! spawns the laptop body + its screen-plane mesh in the bunker,
//! and exposes the `LaptopMaterial` used to display the UI render
//! target on the screen face.

mod components;
mod material;
mod systems;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub use self::material::LaptopMaterial;
use self::systems::{promote_at_laptop_to_state, spawn_laptop};
use crate::PlayingState;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LaptopMaterial>::default());
        app.add_systems(Update, spawn_laptop);
        // Promote `CameraMode::AtLaptop` to `PlayingState::Laptop`
        // so the fullscreen UI swap only happens at the end of the
        // zoom animation. Gated on `resource_exists` because
        // `PlayingState` is a sub-state of `AppState::Playing`.
        app.add_systems(
            Update,
            promote_at_laptop_to_state.run_if(resource_exists::<State<PlayingState>>),
        );
    }
}
