//! Laptop view: the Zone map with areas, bunker, and NPC dots.
//!
//! The laptop is composed of a handful of sub-plugins, each
//! owning one concern:
//!
//! - [`environment`] — terrain, clouds, anomaly shaders, day/night,
//!   the fog overlay mesh, and the CRT post-effect.
//! - [`fog`] — fog of war (player-squad visibility + memory trail).
//! - [`map`] — area disks, borders, bunker marker, relic dots.
//! - [`npcs`] — NPC dot meshes, faction palette, selection rings.
//! - [`hover`] — cursor → tooltip resolution.
//! - [`input`] — keyboard/mouse pan and zoom, `CameraTarget`.
//! - [`ui`] — tab bar, tooltip panel, tab-specific HUDs.
//! - [`visuals`] — sim → visual reactions (tracers, corpse X-marks).

mod environment;
pub(crate) mod fog;
mod hover;
pub(crate) mod input;
pub(crate) mod map;
pub(crate) mod npcs;
mod ui;
mod visuals;

use bevy::prelude::*;

pub use self::npcs::{FactionPalette, SelectedNpc};

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            ui::UiPlugin,
            environment::EnvironmentPlugin,
            fog::FogPlugin,
            visuals::VisualsPlugin,
            map::MapPlugin,
            npcs::NpcsPlugin,
            hover::HoverPlugin,
        ));
        app.add_systems(Startup, setup_camera);
    }
}

/// Marker for the 2D orthographic camera that renders the laptop
/// map view. Only active while the player is on the laptop.
#[derive(Component)]
pub struct LaptopCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        LaptopCamera,
        Camera2d,
        Camera {
            is_active: false,
            order: 1,
            ..default()
        },
        Transform::from_xyz(0.0, -100.0, 1000.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}
