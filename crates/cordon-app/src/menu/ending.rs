//! End-of-run slate. Terminal's outcome stage (and any future
//! ending, bankruptcy etc.) fires an [`EndGameRequest`] carrying a
//! `cause` enum. A handler transitions [`AppState::Ending`] and the
//! overlay reads the stashed cause to render its epitaph text.
//!
//! The "CONTINUE" button resets the run and returns to Menu. Esc
//! is wired as a safety-net alias so the player can never softlock
//! if the button fails to render.

use bevy::prelude::*;
use cordon_core::world::narrative::EndingCause;
use cordon_sim::quest::EndGameRequest;

use super::style::{MenuButton, OverlayButton, spawn_overlay};
use crate::AppState;
use crate::bunker::FpsCamera;
use crate::ui::UiFont;

/// Stashed cause for the current ending. Set when we enter
/// [`AppState::Ending`], read by the slate UI.
#[derive(Resource, Debug, Clone, Copy)]
struct ActiveEndingCause(EndingCause);

impl Default for ActiveEndingCause {
    fn default() -> Self {
        Self(EndingCause::Generic)
    }
}

#[derive(Component)]
struct EndingRoot;

#[derive(Component, Clone, Copy)]
struct ContinueButton;

pub(super) struct EndingPlugin;

impl Plugin for EndingPlugin {
    fn build(&self, app: &mut App) {
        // `EndGameRequest` is registered by cordon-sim's QuestPlugin.
        app.init_resource::<ActiveEndingCause>();
        app.add_systems(
            Update,
            apply_end_game_request.run_if(in_state(AppState::Playing)),
        );
        app.add_systems(OnEnter(AppState::Ending), spawn_ending);
        app.add_systems(OnExit(AppState::Ending), despawn_ending);
        app.add_systems(
            Update,
            (handle_click, handle_esc).run_if(in_state(AppState::Ending)),
        );
    }
}

fn apply_end_game_request(
    mut requests: MessageReader<EndGameRequest>,
    mut cause: ResMut<ActiveEndingCause>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Some(req) = requests.read().last() else {
        return;
    };
    cause.0 = req.cause;
    next_state.set(AppState::Ending);
}

fn spawn_ending(
    mut commands: Commands,
    font: Res<UiFont>,
    cause: Res<ActiveEndingCause>,
    camera_q: Query<Entity, With<FpsCamera>>,
) {
    let Ok(camera) = camera_q.single() else {
        return;
    };
    let epitaph = epitaph_for(cause.0);
    spawn_overlay(
        &mut commands,
        font.0.clone(),
        camera,
        EndingRoot,
        "END OF THE LINE",
        None,
        Some(epitaph),
        vec![OverlayButton::new("CONTINUE", |mut e| {
            e.insert(ContinueButton);
        })],
    );
}

fn despawn_ending(mut commands: Commands, q: Query<Entity, With<EndingRoot>>) {
    for entity in &q {
        commands.entity(entity).despawn();
    }
}

fn handle_click(
    buttons: Query<(&Interaction, &MenuButton), (Changed<Interaction>, With<ContinueButton>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, btn) in &buttons {
        if !matches!(interaction, Interaction::Pressed) || !btn.enabled {
            continue;
        }
        next_state.set(AppState::Menu);
    }
}

/// Esc acts as a fallback for the Continue button — if the button
/// fails to render for any reason, the player isn't softlocked.
fn handle_esc(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    next_state.set(AppState::Menu);
}

/// Map an ending cause to its flavor text. Exhaustive match so a
/// new variant surfaces as a compile error here — no silent
/// fallback to generic flavor.
fn epitaph_for(cause: EndingCause) -> &'static str {
    match cause {
        EndingCause::Terminal => "The pills ran out. The Tenant stayed.",
        EndingCause::Bankruptcy => "The Garrison collected what you couldn't pay.",
        EndingCause::Generic => "The Zone claimed you.",
    }
}
