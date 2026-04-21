//! Main menu overlay — shown on top of the bunker scene while the
//! app is in [`AppState::Menu`]. Offers New Game, Settings
//! (disabled stub), Quit.

use bevy::app::AppExit;
use bevy::prelude::*;

use super::style::{MenuButton, OverlayButton, spawn_overlay};
use crate::AppState;
use crate::bunker::FpsCamera;
use crate::ui::UiFont;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component, Clone, Copy)]
enum MainMenuAction {
    NewGame,
    Quit,
}

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(AppState::Menu), despawn_main_menu);
        app.add_systems(
            Update,
            (ensure_spawned, handle_click).run_if(in_state(AppState::Menu)),
        );
    }
}

/// Self-gates on Menu state, camera-exists, and not-already-spawned.
/// Handles first-boot race (camera spawns same frame as OnEnter),
/// post-ending re-entry, and pause→menu uniformly.
fn ensure_spawned(
    mut commands: Commands,
    font: Res<UiFont>,
    camera_q: Query<Entity, With<FpsCamera>>,
    existing: Query<(), With<MainMenuRoot>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok(camera) = camera_q.single() else {
        return;
    };
    spawn_overlay(
        &mut commands,
        font.0.clone(),
        camera,
        MainMenuRoot,
        "CORDON",
        Some("survival in the zone"),
        None,
        vec![
            OverlayButton::new("NEW GAME", |mut e| {
                e.insert(MainMenuAction::NewGame);
            }),
            OverlayButton::disabled("SETTINGS"),
            OverlayButton::new("QUIT", |mut e| {
                e.insert(MainMenuAction::Quit);
            }),
        ],
    );
}

fn despawn_main_menu(mut commands: Commands, q: Query<Entity, With<MainMenuRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}

fn handle_click(
    buttons: Query<(&Interaction, &MainMenuAction, &MenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action, btn) in &buttons {
        if !matches!(interaction, Interaction::Pressed) || !btn.enabled {
            continue;
        }
        match action {
            MainMenuAction::NewGame => next_state.set(AppState::Playing),
            MainMenuAction::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}
