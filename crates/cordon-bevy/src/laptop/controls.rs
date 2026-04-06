//! Camera controls: zoom, pan.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::AppState;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, zoom_camera.run_if(in_state(AppState::InGame)));
    }
}

fn zoom_camera(
    mut scroll: MessageReader<MouseWheel>,
    mut camera_q: Query<&mut Projection, With<Camera2d>>,
) {
    let delta: f32 = scroll.read().map(|e| e.y).sum();
    if delta == 0.0 {
        return;
    }
    for mut proj in &mut camera_q {
        if let Projection::Orthographic(ref mut ortho) = *proj {
            let factor = 1.0 - delta * 0.1;
            ortho.scale = (ortho.scale * factor).clamp(0.3, 5.0);
        }
    }
}
