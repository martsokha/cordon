pub mod bundles;
pub mod components;
pub mod materials;
mod systems;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub use self::components::MonitorPlacement;
pub use self::materials::CctvMaterial;
use self::systems::{
    apply_cctv_fullscreen, ensure_fullscreen_plane, follow_fps_camera, spawn_cctv,
};
use crate::PlayingState;

pub struct CctvPlugin;

impl Plugin for CctvPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CctvMaterial>::default());
        app.add_systems(
            Update,
            (
                spawn_cctv,
                ensure_fullscreen_plane,
                apply_cctv_fullscreen,
                follow_fps_camera,
            )
                .chain()
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
