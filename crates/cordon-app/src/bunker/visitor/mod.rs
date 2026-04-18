//! Visitor lifecycle: queue, knocking, admit, dialogue, dismiss.
//!
//! Split into:
//! - [`state`] — public types (`Visitor`, `VisitorQueue`,
//!   `VisitorState`, `AdmitVisitor`).
//! - [`lifecycle`] — state-machine transition systems (arrive,
//!   admit, dismiss, preview despawn).
//! - [`audio`] — door SFX (alarm, open, close).
//! - [`ui`] — button glow, cursor lock, interaction observer.

mod audio;
mod lifecycle;
pub mod state;
mod ui;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
pub use state::{Visitor, VisitorQueue, VisitorState};
pub use ui::DoorButton;

use self::audio::AlarmSound;
use crate::{AppState, PauseState, PlayingState};

pub struct VisitorPlugin;

impl Plugin for VisitorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VisitorQueue::default());
        app.insert_resource(VisitorState::Quiet);
        app.add_message::<state::AdmitVisitor>();
        app.add_systems(Startup, audio::load);
        // Visitor arrivals + dismissals run regardless of whether
        // the player is at the laptop or walking around — the
        // door alarm should sound even from the laptop view.
        // Gated on GameDataResource existing (inserted at
        // enter-play, not at app startup).
        app.add_systems(
            Update,
            (
                lifecycle::arrive_next_visitor,
                lifecycle::dismiss_on_dialogue_complete,
                lifecycle::despawn_preview_on_leave_knocking,
            )
                .run_if(resource_exists::<GameDataResource>),
        );
        // Bunker-only: systems that need the FPS camera or the
        // physical interaction button.
        app.add_systems(
            Update,
            (
                lifecycle::apply_admit_visitor,
                ui::update_button_glow,
                ui::update_button_enabled,
                ui::update_cursor_lock,
                ui::attach_door_observer,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
        // Kill the looping alarm whenever any menu overlay is up —
        // runs every frame (cheap when no alarm exists) so a stray
        // alarm that somehow spawns mid-menu can't survive. The
        // alternative is three separate `OnEnter` hooks that all
        // only fire on the transition edge, which is brittle.
        app.add_systems(Update, silence_alarm_during_overlays);
    }
}

fn silence_alarm_during_overlays(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
    pause_state: Option<Res<State<PauseState>>>,
    alarm_q: Query<Entity, With<AlarmSound>>,
) {
    if alarm_q.is_empty() {
        return;
    }
    let overlay_up = !matches!(app_state.get(), AppState::Playing)
        || pause_state.is_some_and(|s| matches!(s.get(), PauseState::Paused));
    if !overlay_up {
        return;
    }
    for entity in &alarm_q {
        commands.entity(entity).despawn();
    }
}
