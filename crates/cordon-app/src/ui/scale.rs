//! Resolution-driven UI scaling.
//!
//! [`UiScale`] is Bevy's built-in multiplier for the UI tree —
//! every pixel dimension in a `Node` (width, height, padding,
//! margin, gap) and every `TextFont::font_size` gets multiplied
//! by this factor before rendering. One number, applied to the
//! whole UI.
//!
//! This plugin keeps `UiScale` in sync with the primary window's
//! logical height so the laptop, dialogue panel, and toasts stay
//! legible at any resolution without per-call-site changes.
//!
//! At [`REFERENCE_HEIGHT`] the scale is `1.0` — what was authored
//! is what's shown. Smaller windows shrink proportionally; larger
//! ones enlarge. Clamped on both ends so tiny windows don't
//! produce unreadable text and 8K displays don't nuke the layout.

use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::{PrimaryWindow, WindowResized};

/// Logical-pixel height at which `UiScale = 1.0`. Picked as
/// 1080p — the most common desktop target — so most players see
/// the UI at its authored size.
const REFERENCE_HEIGHT: f32 = 1080.0;

/// Minimum scale. Below this, text and buttons become too small
/// to read comfortably.
const MIN_SCALE: f32 = 0.7;

/// Maximum scale. Above this, fixed-size panels start clipping
/// off the edges of the window.
const MAX_SCALE: f32 = 2.0;

/// Changes smaller than this threshold are ignored to avoid
/// churning `UiScale` every frame on sub-pixel resize tremors
/// (dragging the window edge, for example).
const EPSILON: f32 = 0.01;

pub struct UiScalePlugin;

impl Plugin for UiScalePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UiScale(1.0));
        // Startup sync picks up the initial window size before
        // any `WindowResized` has fired; `on_resize` handles
        // subsequent user drags / monitor changes.
        app.add_systems(Startup, sync_on_startup);
        app.add_systems(Update, on_resize);
    }
}

fn sync_on_startup(windows: Query<&Window, With<PrimaryWindow>>, mut ui_scale: ResMut<UiScale>) {
    if let Ok(window) = windows.single() {
        apply(&mut ui_scale, window.height());
    }
}

fn on_resize(
    mut events: MessageReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    // Drain every event but only act on the last — a drag
    // produces dozens of intermediate sizes per frame and we
    // only care about the current one.
    if events.read().last().is_none() {
        return;
    }
    if let Ok(window) = windows.single() {
        apply(&mut ui_scale, window.height());
    }
}

fn apply(ui_scale: &mut UiScale, window_height: f32) {
    let target = (window_height / REFERENCE_HEIGHT).clamp(MIN_SCALE, MAX_SCALE);
    if (ui_scale.0 - target).abs() > EPSILON {
        ui_scale.0 = target;
    }
}
