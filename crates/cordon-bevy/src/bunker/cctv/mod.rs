pub mod bundles;
pub mod components;
pub mod materials;
mod systems;

use bevy::prelude::*;

pub use self::components::*;
pub use self::materials::CctvMaterial;
use crate::PlayingState;

pub struct CctvPlugin;

impl Plugin for CctvPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<CctvMaterial>::default());
        app.add_systems(
            Update,
            (
                systems::spawn_cctv,
                systems::ensure_fullscreen_plane,
                systems::apply_cctv_fullscreen,
                systems::follow_fps_camera,
            )
                .chain()
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
