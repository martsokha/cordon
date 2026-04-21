//! Dev-time overlays, split by concern so each piece can be
//! turned on or off independently via Cargo features.
//!
//! - `development` — asset hot-reload + keybindings that mutate
//!   state (F3 fog, F4 time-scale). Base dev feature.
//! - `diagnostic` — FPS counter + Bevy's frame-time / entity-count /
//!   log diagnostics plugins. Observational only. Implies
//!   `development`.
//! - `inspector` — `bevy_inspector_egui` world inspector (F1). Heavy
//!   egui dependency, so it gets its own toggle. Implies
//!   `development`.
//!
//! [`DevPlugin`] composes whichever sub-plugins are enabled by the
//! active feature set. The whole module is additionally gated behind
//! `debug_assertions` at the `main.rs` `mod dev;` declaration, so
//! release builds skip the compile cost of any of this entirely.

#[cfg(feature = "development")]
mod cheat;
#[cfg(feature = "diagnostic")]
mod diagnostic;
#[cfg(feature = "inspector")]
mod inspector;

use bevy::prelude::*;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(feature = "diagnostic")]
        _app.add_plugins(diagnostic::DiagnosticPlugin);
        #[cfg(feature = "inspector")]
        _app.add_plugins(inspector::InspectorPlugin);
        #[cfg(feature = "development")]
        _app.add_plugins(cheat::CheatPlugin);
    }
}
