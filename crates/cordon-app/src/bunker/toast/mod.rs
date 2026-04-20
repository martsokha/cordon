//! Corner toast notifications: icon + text that fade in, hold,
//! and fade out. Subscribes directly to sim-layer messages and
//! resource changes — emitting systems don't know toasts exist.

mod systems;

use bevy::prelude::*;
pub(crate) use systems::reset_toast_queue;

use crate::{AppState, PauseState};

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::load_atlas);
        // Gated on `AppState::Playing` (so no toasts during main
        // menu / ending) and `PauseState::Running` (no spawns or
        // fade-progress during pause — the alpha animation reads
        // `Time<Real>` which would keep ticking regardless, so we
        // have to gate explicitly). Runs in any `PlayingState`:
        // a quest toast fired while the player is on the laptop
        // should still surface, the toast UI targets the FPS
        // camera either way.
        app.add_systems(
            Update,
            (
                systems::on_radio_broadcast,
                systems::on_daily_expenses,
                systems::on_decision_recorded,
                systems::on_quest_started,
                systems::on_quest_progress,
                systems::spawn_toasts,
                systems::animate_toasts,
            )
                .chain()
                .run_if(in_state(AppState::Playing))
                .run_if(in_state(PauseState::Running)),
        );
    }
}
