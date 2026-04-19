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

use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

pub use self::npcs::{FactionPalette, SelectedNpc};
use crate::PlayingState;

/// Resolution of the laptop screen's render target. Matches the
/// 4:3 aspect of the Laptop_02 model's screen face; the size is a
/// balance between legibility (text must read at ~1.5 m viewing
/// distance in the bunker) and cost (the whole map UI rasterises
/// into this every frame).
pub const LAPTOP_RT_WIDTH: u32 = 1024;
pub const LAPTOP_RT_HEIGHT: u32 = 768;

/// Handle to the image the laptop UI camera renders into. The
/// bunker's laptop prop samples this as a texture on its screen
/// face via `LaptopMaterial`.
#[derive(Resource, Clone)]
pub struct LaptopScreenImage(pub Handle<Image>);

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
        // Swap the camera's render target based on playing state:
        // bunker → off-screen laptop screen image (desk
        // projection), laptop → window (crisp fullscreen UI at
        // native resolution, same as before the plane-projection
        // work landed).
        app.add_systems(
            Update,
            swap_render_target.run_if(resource_exists::<State<PlayingState>>),
        );
    }
}

fn swap_render_target(
    state: Res<State<PlayingState>>,
    screen_image: Res<LaptopScreenImage>,
    mut cam_q: Query<&mut RenderTarget, With<LaptopCamera>>,
) {
    let Ok(mut target) = cam_q.single_mut() else {
        return;
    };
    // Pin the intended target every frame. Change-guard the
    // write so the render pipeline isn't dirtied unless the
    // state actually moved, but don't gate the whole system on
    // `state.is_changed()` — if the target ever desyncs (e.g.
    // a mid-frame mutation from another system), the next
    // frame snaps it back.
    let desired_is_window = matches!(state.get(), PlayingState::Laptop);
    let current_is_window = matches!(*target, RenderTarget::Window(_));
    if desired_is_window == current_is_window {
        return;
    }
    *target = if desired_is_window {
        RenderTarget::default()
    } else {
        RenderTarget::Image(ImageRenderTarget {
            handle: screen_image.0.clone(),
            scale_factor: 1.0,
        })
    };
}

/// Marker for the 2D orthographic camera that renders the laptop
/// map view into `LaptopScreenImage`.
#[derive(Component)]
pub struct LaptopCamera;

fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_target_texture(
        LAPTOP_RT_WIDTH,
        LAPTOP_RT_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let image_handle = images.add(image);
    commands.insert_resource(LaptopScreenImage(image_handle.clone()));

    commands.spawn((
        LaptopCamera,
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.04, 0.04, 0.06)),
            ..default()
        },
        // Render into the laptop screen texture instead of the
        // window. The bunker's laptop prop samples the texture
        // on its screen face, so the UI is always "live" on the
        // desk regardless of whether the player is in laptop
        // mode or walking around.
        RenderTarget::Image(ImageRenderTarget {
            handle: image_handle,
            scale_factor: 1.0,
        }),
        Transform::from_xyz(0.0, -100.0, 1000.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}
