use bevy::prelude::*;

use crate::bunker::resources::CameraMode;
use crate::laptop::LaptopCamera;

/// Sync the laptop camera's active state with CameraMode. The
/// laptop camera is only active while the bunker camera has
/// finished its zoom animation and is in AtLaptop mode.
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
