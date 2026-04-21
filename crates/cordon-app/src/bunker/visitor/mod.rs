//! Visitor lifecycle: queue, knocking, admit, dialogue, dismiss,
//! step-away / resume.
//!
//! Split into:
//! - [`state`] — public types (`Visitor`, `VisitorQueue`,
//!   `VisitorState`, `AdmitVisitor`, `PendingStepAway`).
//! - [`lifecycle`] — state-machine transition systems (arrive,
//!   admit, dismiss-or-wait, preview despawn, resume-on-interact).
//! - [`audio`] — door SFX (alarm, open, close).
//! - [`ui`] — door button glow, cursor lock.

mod audio;
mod lifecycle;
pub mod state;
mod ui;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
pub use lifecycle::reset_visitor_state;
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
                lifecycle::update_waiting_interactable,
                ui::update_button_glow,
                ui::update_button_enabled,
                ui::update_cursor_lock,
                ui::attach_door_observer,
                attach_visitor_interact_observer,
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

/// Attach the visitor-interact observer to the sprite the
/// moment it enters `Waiting`. A single observer is bound per
/// sprite; the sprite is despawned on dismiss, so the observer
/// goes with it. `already_attached` remembers the sprite entity
/// we last observed so we don't double-bind if the state
/// change-detect fires without the sprite actually changing
/// (e.g. on re-entry from Inside → Waiting with the same sprite).
fn attach_visitor_interact_observer(
    mut commands: Commands,
    state: Res<state::VisitorState>,
    mut already_attached: Local<Option<Entity>>,
) {
    if !state.is_changed() {
        return;
    }
    match &*state {
        state::VisitorState::Waiting { sprite, .. } => {
            if *already_attached == Some(*sprite) {
                return;
            }
            commands
                .entity(*sprite)
                .observe(lifecycle::on_visitor_interact);
            *already_attached = Some(*sprite);
        }
        state::VisitorState::Quiet => {
            *already_attached = None;
        }
        _ => {}
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
