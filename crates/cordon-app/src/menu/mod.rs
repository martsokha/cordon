//! Menu overlays that sit on top of the bunker scene: main menu,
//! pause menu, ending slate.
//!
//! All three share a common terminal-style aesthetic (monospace,
//! dim background, centered column of buttons). Run lifecycle
//! (resource seeding + entity despawn) is handled by the
//! [`crate::lifecycle`] module via Bevy's state-scoped entities:
//! "New Game" and "Main Menu" just transition the app state, and
//! the despawn cascade + resource reseed happen as state hooks.
//!
//! The bunker scene itself is spawned once at `OnExit(Loading)`
//! and persists forever; menu overlays just sit on top. When the
//! sim should freeze (menu shown, pause active, dialog open,
//! ending shown) a dedicated system scales `SimSpeed` to zero so
//! every system that reads `Time<Sim>` freezes in lockstep.

mod ending;
mod main_menu;
mod pause;
mod style;
mod timescale;

use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            timescale::TimeScalePlugin,
            main_menu::MainMenuPlugin,
            pause::PausePlugin,
            ending::EndingPlugin,
        ));
        // Hover tint applies to every menu button across the three
        // overlays, so we register it once at the parent level rather
        // than three times in each sub-plugin. `Changed<Interaction>`
        // filter inside keeps the cost near zero when nothing is being
        // hovered.
        app.add_systems(Update, style::update_button_hover);
    }
}
