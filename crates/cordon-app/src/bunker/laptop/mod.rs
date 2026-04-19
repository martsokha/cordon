//! Laptop interaction feature: owns the LaptopObject component,
//! spawns the laptop body + its screen-plane mesh in the bunker,
//! and exposes the `LaptopMaterial` used to display the UI render
//! target on the screen face.
//!
//! The PlayingState flip from `Bunker` → `Laptop` is driven by
//! the fade overlay (see [`crate::bunker::fade`]) at the fade
//! peak — not by this module. The interact observer here just
//! kicks the zoom animation + starts the fade.

mod components;
mod material;
mod systems;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub use self::material::LaptopMaterial;
use self::systems::spawn_laptop;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LaptopMaterial>::default());
        app.add_systems(Update, spawn_laptop);
    }
}
