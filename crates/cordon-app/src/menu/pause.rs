//! Pause overlay. Esc toggles it while the player is in
//! [`AppState::Playing`] (and not mid-dialogue — an active dialog
//! owns Esc). Offers Resume, Settings (disabled), Main Menu, Quit.
//!
//! "Main Menu" resets the run and returns to [`AppState::Menu`].
//! There is no save/load yet, so the in-flight run is discarded.

use bevy::app::AppExit;
use bevy::prelude::*;

use super::style::{MenuButton, OverlayButton, spawn_overlay};
use crate::bunker::FpsCamera;
use crate::bunker::radio::ListeningToRadio;
use crate::bunker::resources::CurrentDialogue;
use crate::ui::UiFont;
use crate::{AppState, PauseState};

#[derive(Component)]
struct PauseRoot;

#[derive(Component, Clone, Copy)]
enum PauseAction {
    Resume,
    MainMenu,
    Quit,
}

pub(super) struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(PauseState::Paused), spawn_pause_menu);
        app.add_systems(OnExit(PauseState::Paused), despawn_pause_menu);
        app.add_systems(
            Update,
            (
                toggle_pause.run_if(in_state(AppState::Playing)),
                handle_click.run_if(in_state(PauseState::Paused)),
            ),
        );
    }
}

fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    pause_state: Res<State<PauseState>>,
    dialogue: Res<CurrentDialogue>,
    listening: Res<ListeningToRadio>,
    mut next_pause: ResMut<NextState<PauseState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    // Active dialogue owns Esc — pausing mid-line would steal focus
    // from the dialogue runner.
    if !matches!(*dialogue, CurrentDialogue::Idle) {
        return;
    }
    // Radio listening mode owns Esc end-to-end: during the static
    // gap `CurrentDialogue::Idle` is true, but Esc there should
    // exit listening, not open the pause menu. Also blocks pausing
    // while a broadcast dialog is open — the radio UI owns the
    // player's focus for the whole session.
    if listening.active {
        return;
    }
    let next = match pause_state.get() {
        PauseState::Running => PauseState::Paused,
        PauseState::Paused => PauseState::Running,
    };
    next_pause.set(next);
}

fn spawn_pause_menu(
    mut commands: Commands,
    font: Res<UiFont>,
    camera_q: Query<Entity, With<FpsCamera>>,
) {
    let Ok(camera) = camera_q.single() else {
        return;
    };
    spawn_overlay(
        &mut commands,
        font.0.clone(),
        camera,
        PauseRoot,
        "PAUSED",
        None,
        None,
        vec![
            OverlayButton::new("RESUME", |mut e| {
                e.insert(PauseAction::Resume);
            }),
            OverlayButton::disabled("SETTINGS"),
            OverlayButton::new("MAIN MENU", |mut e| {
                e.insert(PauseAction::MainMenu);
            }),
            OverlayButton::new("QUIT", |mut e| {
                e.insert(PauseAction::Quit);
            }),
        ],
    );
}

fn despawn_pause_menu(mut commands: Commands, q: Query<Entity, With<PauseRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}

fn handle_click(
    buttons: Query<(&Interaction, &PauseAction, &MenuButton), Changed<Interaction>>,
    mut next_app: ResMut<NextState<AppState>>,
    mut next_pause: ResMut<NextState<PauseState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action, btn) in &buttons {
        if !matches!(interaction, Interaction::Pressed) || !btn.enabled {
            continue;
        }
        match action {
            PauseAction::Resume => next_pause.set(PauseState::Running),
            PauseAction::MainMenu => {
                // Unpause first so PauseState resets to Running when we
                // re-enter Playing. Without this, a next New Game would
                // boot straight into the pause overlay.
                next_pause.set(PauseState::Running);
                next_app.set(AppState::Menu);
            }
            PauseAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
