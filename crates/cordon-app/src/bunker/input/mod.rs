//! Bunker input: FPS controls, cursor management, and input-
//! triggered feedback (footstep audio).

pub(crate) mod controller;
mod footsteps;
mod systems;

use bevy::prelude::*;

use crate::{AppState, PauseState, PlayingState};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(controller::ControllerPlugin);
        footsteps::plugin(app);
        app.add_systems(OnEnter(PlayingState::Bunker), systems::grab_cursor);
        app.add_systems(OnEnter(PlayingState::Laptop), systems::hide_interact_prompt);
        // Release the cursor whenever a menu overlay takes over so the
        // player can click the buttons. The laptop state already
        // handles its own cursor via the laptop UI.
        app.add_systems(OnEnter(AppState::Menu), systems::release_cursor);
        app.add_systems(OnEnter(AppState::Ending), systems::release_cursor);
        app.add_systems(OnEnter(PauseState::Paused), systems::release_cursor);
        // Re-grab only when unpausing into the bunker; the laptop
        // state wants the cursor free for map UI. Intentional split
        // rather than "regrab iff Bunker" guesswork.
        app.add_systems(OnExit(PauseState::Paused), sync_cursor_for_playing_state);
    }
}

/// Cursor discipline on pause exit: Bunker locks the cursor for FPS
/// look, Laptop leaves it free for UI. Also re-runs after any pause
/// round-trip so we don't depend on the pre-pause cursor state
/// surviving the overlay's `release_cursor` call.
///
/// Skips entirely if we're leaving `Playing` to `Menu` / `Ending`,
/// since those states own cursor release via their own `OnEnter`
/// hooks. Without this gate, clicking "Main Menu" from pause would
/// flash a locked cursor for one frame before the Menu state
/// released it again.
fn sync_cursor_for_playing_state(
    app_state: Res<State<AppState>>,
    playing: Option<Res<State<PlayingState>>>,
    cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    // `PlayingState` is a sub-state of `AppState::Playing`, so when
    // "Main Menu" from pause flips us to `AppState::Menu` the
    // OnExit(Paused) fires in the same frame the sub-state is being
    // torn down — `State<PlayingState>` no longer exists. Early-out
    // in that case; the Menu's own OnEnter releases the cursor.
    if !matches!(app_state.get(), AppState::Playing) {
        return;
    }
    let Some(playing) = playing else {
        return;
    };
    match playing.get() {
        PlayingState::Bunker => systems::grab_cursor(cursor_q),
        PlayingState::Laptop => systems::release_cursor(cursor_q),
    }
}
