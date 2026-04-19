//! Screen-fade overlay for laptop and CCTV transitions.
//!
//! Opening the laptop and entering the CCTV both need to swap
//! the rendered content (window render-target swap for laptop,
//! `CameraMode::AtCctv` for CCTV). Both swaps are single-frame
//! cuts, which look ugly on their own. A 180 ms out / 180 ms in
//! black fade hides the cut: callers [`start`] a fade with an
//! [`Action`], the fade driver applies the action at the peak
//! (alpha = 1), then fades back in.
//!
//! The overlay also eats pointer events while active so rapid
//! clicks don't slip through to the laptop UI during the fade.

use bevy::prelude::*;

use super::resources::CameraMode;
use crate::PlayingState;

/// Half of the total fade duration, in seconds. Out takes this
/// long, then the action fires at alpha = 1, then in takes this
/// long. 180 ms reads as "decisive" without feeling sluggish.
const FADE_HALF_SECS: f32 = 0.18;

/// What the fade driver should do when the overlay hits full
/// black. One of these is captured on [`start`] and applied
/// during the single-frame transition at peak.
#[derive(Debug, Clone)]
pub enum Action {
    /// Enter the laptop UI. At peak: set `CameraMode::AtLaptop`
    /// (so `animate_camera` pins the camera at `LAPTOP_VIEW_POS`)
    /// and flip `PlayingState::Laptop` to trigger the render-
    /// target swap. Both happen behind full black, so the
    /// player only sees the already-parked view on fade-in.
    EnterLaptop { saved_transform: Transform },
    /// Leave the laptop back to the bunker. At peak: flip
    /// `PlayingState::Bunker`; `start_free_look` then sets the
    /// camera to `Returning` which snaps to the saved transform
    /// under the remaining in-fade.
    ExitLaptop,
    /// Enter the CCTV fullscreen view. At peak: set
    /// `CameraMode::AtCctv`.
    EnterCctv { saved_transform: Transform },
    /// Leave the CCTV view back to the bunker. At peak: set
    /// `CameraMode::Returning`.
    ExitCctv { saved_transform: Transform },
}

/// Drives the overlay's alpha. `Idle` means the overlay is
/// transparent and the driver is quiescent. `Out`/`In` count
/// seconds up to [`FADE_HALF_SECS`] then transition.
#[derive(Resource, Debug, Clone, Default)]
pub enum Fade {
    #[default]
    Idle,
    Out {
        elapsed: f32,
        action: Action,
    },
    In {
        elapsed: f32,
    },
}

/// Kick off a fade. No-op if one is already running — avoids
/// stacking actions if the player mashes E.
pub fn start(fade: &mut Fade, action: Action) {
    if matches!(*fade, Fade::Idle) {
        *fade = Fade::Out {
            elapsed: 0.0,
            action,
        };
    }
}

/// Whether a fade is currently running. Input systems check
/// this to swallow E/Esc during the transition so the player
/// can't queue a second toggle mid-fade.
pub fn is_active(fade: &Fade) -> bool {
    !matches!(fade, Fade::Idle)
}

#[derive(Component)]
struct FadeOverlay;

pub struct FadePlugin;

impl Plugin for FadePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fade>();
        app.add_systems(Startup, spawn_overlay);
        app.add_systems(Update, drive_fade);
    }
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        FadeOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        // Topmost so it covers both the bunker 3D view and the
        // laptop UI tree. UI layering is by draw order within a
        // root, so `GlobalZIndex` beats sibling `ZIndex`es.
        GlobalZIndex(i32::MAX),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Start non-interactive so the invisible overlay doesn't
        // swallow clicks on the laptop UI while idle. Flipped to
        // pickable while a fade runs (see `drive_fade`).
        Pickable::IGNORE,
    ));
}

fn drive_fade(
    time: Res<Time<Real>>,
    mut fade: ResMut<Fade>,
    mut next_state: ResMut<NextState<PlayingState>>,
    mut camera_mode: ResMut<CameraMode>,
    mut overlay_q: Query<(&mut BackgroundColor, &mut Pickable), With<FadeOverlay>>,
) {
    let Ok((mut bg, mut pickable)) = overlay_q.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    let alpha = match &mut *fade {
        Fade::Idle => 0.0,
        Fade::Out { elapsed, .. } => {
            *elapsed += dt;
            (*elapsed / FADE_HALF_SECS).min(1.0)
        }
        Fade::In { elapsed } => {
            *elapsed += dt;
            1.0 - (*elapsed / FADE_HALF_SECS).min(1.0)
        }
    };
    bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);

    // Swallow pointer events while the overlay is visible so a
    // click during the 180 ms in-fade can't hit laptop UI
    // buttons that are coming back into view.
    let idle = matches!(*fade, Fade::Idle);
    if pickable.should_block_lower == idle {
        *pickable = if idle {
            Pickable::IGNORE
        } else {
            Pickable::default()
        };
    }

    // State transition at peak: on `Out` completing, apply the
    // captured action at full black and start the `In`. On `In`
    // completing, drop back to `Idle`. Actions that need extra
    // frames (e.g. the laptop's render-target swap running off
    // `OnEnter(PlayingState::Laptop)`) resolve under the
    // remaining black frames of `In`.
    match &*fade {
        Fade::Out { elapsed, .. } if *elapsed >= FADE_HALF_SECS => {
            let Fade::Out { action, .. } = std::mem::replace(&mut *fade, Fade::Idle) else {
                unreachable!("guarded by the outer match");
            };
            apply_peak(&action, &mut next_state, &mut camera_mode);
            *fade = Fade::In { elapsed: 0.0 };
        }
        Fade::In { elapsed } if *elapsed >= FADE_HALF_SECS => {
            *fade = Fade::Idle;
        }
        _ => {}
    }
}

fn apply_peak(
    action: &Action,
    next_state: &mut NextState<PlayingState>,
    camera_mode: &mut CameraMode,
) {
    match action {
        Action::EnterLaptop { saved_transform } => {
            *camera_mode = CameraMode::AtLaptop {
                saved_transform: *saved_transform,
            };
            *next_state = NextState::Pending(PlayingState::Laptop);
        }
        Action::ExitLaptop => {
            *next_state = NextState::Pending(PlayingState::Bunker);
            // `start_free_look` (OnEnter(PlayingState::Bunker))
            // moves the camera to `Returning` from whatever
            // `AtLaptop` transform is saved, so we don't need
            // to set it here.
        }
        Action::EnterCctv { saved_transform } => {
            *camera_mode = CameraMode::AtCctv {
                saved_transform: *saved_transform,
            };
        }
        Action::ExitCctv { saved_transform } => {
            *camera_mode = CameraMode::Returning(*saved_transform);
        }
    }
}
