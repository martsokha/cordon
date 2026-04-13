pub mod bundles;
pub mod components;
pub mod materials;
mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub use self::components::*;
pub use self::materials::CctvMaterial;

pub struct CctvPlugin;

impl Plugin for CctvPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<CctvMaterial>::default());
        app.add_systems(
            Update,
            (
                systems::ensure_fullscreen_plane,
                systems::apply_cctv_fullscreen,
                systems::follow_fps_camera,
            )
                .chain()
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
