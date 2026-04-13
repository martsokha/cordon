//! Laptop interaction feature: owns the LaptopObject component and
//! manages the external laptop camera lifecycle so the bunker's
//! camera module doesn't need to reach into `crate::laptop`.

mod components;
mod systems;

use bevy::prelude::*;

pub use self::components::LaptopObject;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (systems::sync_laptop_camera, systems::spawn_laptop));
    }
}
