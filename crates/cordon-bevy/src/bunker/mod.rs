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
mod particles;
mod props;
pub mod resources;
mod rooms;
mod systems;
mod textures;
mod visitor;

use bevy::prelude::*;

pub use self::components::FpsCamera;
pub use self::resources::{BunkerSpawned, CameraMode};
// Re-exported only when the steam feature is on: that's the
// only consumer of `VisitorState` outside this module.
#[cfg(feature = "steam")]
pub use self::visitor::VisitorState;
pub use self::visitor::{Visitor, VisitorQueue};
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
            particles::BunkerParticlesPlugin,
        ));
        // SSAO: `ScreenSpaceAmbientOcclusionPlugin` is registered
        // by `DefaultPlugins::PbrPlugin` already; we just attach
        // the `ScreenSpaceAmbientOcclusion` component to the FPS
        // camera in `spawn_camera` to enable the effect.
        // Observer: turns `PropPlacement` components into real
        // prop entities. Registering once here means every room
        // can just `commands.spawn(PropPlacement::new(...))` —
        // no need to thread `AssetServer` through.
        app.add_observer(geometry::resolve_prop_placement);

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
            (
                interaction::update_prompt,
                interaction::interact,
                // Reactive rack spawner: watches `Player` and
                // backfills hall racks when the player installs
                // a rack upgrade after the bunker was first
                // built. Gated by `BunkerSpawned` so the
                // initial `rooms::hall::spawn` inside
                // `spawn_bunker` handles the first fill.
                rooms::hall::sync_hall_racks.run_if(resource_exists::<BunkerSpawned>),
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
