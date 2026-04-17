//! Corner toast notifications: icon + text that fade in, hold,
//! and fade out. Subscribes directly to sim-layer messages and
//! resource changes — emitting systems don't know toasts exist.

mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::load_atlas);
        app.add_systems(
            Update,
            (
                systems::on_radio_broadcast,
                systems::on_daily_expenses,
                systems::on_standing_change,
                systems::spawn_toasts,
                systems::animate_toasts,
            )
                .chain()
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
