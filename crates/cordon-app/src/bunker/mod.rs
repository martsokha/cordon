//! 3D bunker scene with FPS camera. Visitor dialogue lives here too —
//! the player meets visitors at the counter inside the bunker, not on
//! the laptop map.

mod ambient;
pub(crate) mod camera;
pub mod cctv;
pub mod dialogue;
pub mod fade;
pub mod geometry;
mod input;
pub mod interaction;
pub mod laptop;
pub mod lighting;
mod particles;
mod pills;
mod props;
pub mod rack;
mod radio;
pub mod resources;
mod rooms;
mod sleep;
mod systems;
mod textures;
pub mod toast;
mod visitor;

use bevy::prelude::*;

pub use self::camera::FpsCamera;
pub use self::resources::{BunkerSpawned, CameraMode};
// Re-exported only when the steam feature is on: that's the
// only consumer of `VisitorState` outside this module.
#[cfg(feature = "steam")]
pub use self::visitor::VisitorState;
pub use self::visitor::{Visitor, VisitorQueue, reset_visitor_state};
use crate::{AppState, PauseState, PlayingState};

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ambient::AmbientPlugin,
            input::InputPlugin,
            dialogue::DialoguePlugin,
            visitor::VisitorPlugin,
            cctv::CctvPlugin,
            fade::FadePlugin,
            laptop::LaptopPlugin,
            lighting::FlickerPlugin,
            particles::BunkerParticlesPlugin,
            pills::PillsPlugin,
            radio::RadioPlugin,
            rack::RackPlugin,
            sleep::SleepPlugin,
            toast::ToastPlugin,
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
            // Dim ambient so the zones between point-light
            // fixtures fall into proper shadow — makes the
            // bunker read as lived-in and tunnel-like instead
            // of uniformly lit. Nudged up slightly so the
            // shadowed corners aren't pure black.
            brightness: 85.0,
            ..default()
        });
        app.add_systems(OnEnter(PlayingState::Bunker), camera::enable_bunker_camera);
        // Zoom now starts from the laptop interact observer (see
        // `bunker/laptop/systems.rs`). The state transition only
        // happens at the end of the zoom, not on click.
        app.add_systems(OnEnter(PlayingState::Bunker), camera::start_free_look);
        app.add_systems(Update, camera::animate_camera);
        // Bunker spawns once when loading completes, persists across
        // menu / play / ending transitions. The scene serves as a
        // backdrop for the main menu and pause overlays.
        app.add_systems(
            OnEnter(AppState::Menu),
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
                rooms::command::sync_command_listening_device
                    .run_if(resource_exists::<BunkerSpawned>),
            )
                .run_if(in_state(PlayingState::Bunker))
                .run_if(in_state(PauseState::Running)),
        );
    }
}
