use bevy::prelude::*;

use super::components::LaptopObject;
use crate::PlayingState;
use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::{CameraMode, LaptopPlacement};
use crate::laptop::LaptopCamera;

pub(super) fn sync_laptop_camera(
    mode: Res<CameraMode>,
    mut laptop_cam: Query<&mut Camera, With<LaptopCamera>>,
) {
    let should_be_active = matches!(*mode, CameraMode::AtLaptop { .. });
    for mut cam in &mut laptop_cam {
        if cam.is_active != should_be_active {
            cam.is_active = should_be_active;
        }
    }
}

pub(super) fn spawn_laptop(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placement: Option<Res<LaptopPlacement>>,
) {
    let Some(placement) = placement else { return };
    let scene: Handle<Scene> = asset_server.load("models/interior/Laptop.glb#Scene0");
    commands
        .spawn((
            LaptopObject,
            Interactable {
                prompt: "[E] Use Laptop",
                enabled: true,
            },
            SceneRoot(scene),
            Transform::from_translation(placement.pos).with_rotation(placement.rot),
        ))
        .observe(
            |_trigger: On<Interact>, mut next_state: ResMut<NextState<PlayingState>>| {
                *next_state = NextState::Pending(PlayingState::Laptop);
            },
        );
    commands.remove_resource::<LaptopPlacement>();
}
