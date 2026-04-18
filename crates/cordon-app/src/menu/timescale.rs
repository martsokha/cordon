//! Freeze the sim clock whenever the player is in a menu, paused,
//! or mid-dialogue.
//!
//! [`SimSpeed`] is the single knob the sim reads every frame via
//! `Time<Sim>`. Setting it to 0.0 pauses movement, combat, day
//! progression, event cadence — everything that advances time.
//! This module owns that knob for non-gameplay states; the `cheat`
//! feature's F4 time-scale cycling only writes to it while the
//! sim is otherwise running (see the `is_gameplay_active` gate
//! below).

use bevy::prelude::*;
use cordon_sim::resources::SimSpeed;

use crate::bunker::resources::CurrentDialogue;
use crate::{AppState, PauseState};

/// Tracks whether we're currently forcing the sim to a halt so we
/// can restore the previous speed when gameplay resumes. Without
/// this, the cheat time-scale (1× / 4× / 16× / 64×) would get
/// clobbered on every menu toggle.
#[derive(Resource, Debug, Default, Clone, Copy)]
struct FrozenBy {
    /// Speed we captured before forcing the halt. `None` means we
    /// aren't holding the sim frozen right now.
    previous: Option<f64>,
}

pub(super) struct TimeScalePlugin;

impl Plugin for TimeScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrozenBy>();
        app.add_systems(Update, apply_freeze);
    }
}

/// Gameplay is "active" only when the player is in the Playing
/// state, not paused, and no dialogue is up. Any other combination
/// halts the sim.
fn is_gameplay_active(
    app_state: &State<AppState>,
    pause_state: Option<&State<PauseState>>,
    dialogue: &CurrentDialogue,
) -> bool {
    if !matches!(app_state.get(), AppState::Playing) {
        return false;
    }
    if !matches!(pause_state.map(|s| s.get()), Some(PauseState::Running)) {
        return false;
    }
    matches!(dialogue, CurrentDialogue::Idle)
}

fn apply_freeze(
    mut frozen: ResMut<FrozenBy>,
    mut speed: ResMut<SimSpeed>,
    app_state: Res<State<AppState>>,
    pause_state: Option<Res<State<PauseState>>>,
    dialogue: Res<CurrentDialogue>,
) {
    let should_run = is_gameplay_active(&app_state, pause_state.as_deref(), &dialogue);
    match (should_run, frozen.previous) {
        (false, None) => {
            // Entering a freeze. Remember the current speed so the
            // cheat scale survives the pause round-trip.
            frozen.previous = Some(speed.0);
            speed.0 = 0.0;
        }
        (true, Some(prev)) => {
            speed.0 = prev;
            frozen.previous = None;
        }
        _ => {}
    }
}
