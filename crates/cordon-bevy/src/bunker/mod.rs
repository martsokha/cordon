//! 3D bunker scene with FPS camera. Visitor dialogue lives here too —
//! the player meets visitors at the counter inside the bunker, not on
//! the laptop map.

mod camera;
pub mod cctv;
pub mod components;
pub mod dialogue;
pub mod geometry;
mod input;
pub mod interaction;
pub mod laptop;
pub mod lighting;
mod props;
pub mod resources;
mod rooms;
mod systems;
mod visitor;

use bevy::prelude::*;

pub use self::components::*;
pub use self::resources::{BunkerSpawned, CameraMode};
pub use self::rooms::ANTECHAMBER_VISITOR_POS;
pub use self::visitor::{Visitor, VisitorQueue, VisitorState};
use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            dialogue::DialoguePlugin,
            visitor::VisitorPlugin,
            cctv::CctvPlugin,
            laptop::LaptopPlugin,
        ));
        app.insert_resource(CameraMode::Free);
        app.insert_resource(bevy::light::GlobalAmbientLight {
            color: Color::srgb(0.9, 0.85, 0.70),
            brightness: 80.0,
            ..default()
        });
        app.add_systems(OnEnter(PlayingState::Bunker), camera::enable_bunker_camera);
        app.add_systems(OnEnter(PlayingState::Laptop), camera::start_laptop_zoom);
        app.add_systems(OnEnter(PlayingState::Bunker), camera::start_free_look);
        app.add_systems(Update, camera::animate_camera);
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            systems::spawn_bunker.run_if(not(resource_exists::<BunkerSpawned>)),
        );
        app.add_systems(
            Update,
            (interaction::update_prompt, interaction::interact)
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
