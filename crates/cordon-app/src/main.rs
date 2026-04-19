#![forbid(unsafe_code)]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod bunker;
#[cfg(all(debug_assertions, feature = "development"))]
mod dev;
mod fonts;
mod laptop;
mod lifecycle;
mod locale;
mod menu;
mod quest;
#[cfg(feature = "steam")]
pub mod steam;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use cordon_data::gamedata::GameDataPlugin;
use cordon_sim::plugin::CordonSimPlugin;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    /// Load game data + assets.
    #[default]
    Loading,
    /// Main menu overlay on top of the bunker scene. Default after
    /// loading completes; also where we return after an ending.
    Menu,
    /// Active run. Gameplay systems gate on this.
    Playing,
    /// End-of-run slate (Terminal, bankruptcy, etc.). Transient — the
    /// Continue button returns to Menu.
    Ending,
}

#[derive(SubStates, Default, Clone, Eq, PartialEq, Hash, Debug)]
#[source(AppState = AppState::Playing)]
pub enum PlayingState {
    #[default]
    Bunker,
    Laptop,
}

/// Orthogonal to [`PlayingState`]: whether the sim is actively
/// progressing. Pause is a modal overlay (`Esc` from any
/// `PlayingState`), so it lives as its own sub-state tied to
/// [`AppState::Playing`]. Gameplay systems that should freeze during
/// pause, dialog, or menus gate on `in_state(PauseState::Running)`.
#[derive(SubStates, Default, Clone, Eq, PartialEq, Hash, Debug)]
#[source(AppState = AppState::Playing)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Cordon".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            }),
    )
    .init_state::<AppState>()
    .add_sub_state::<PlayingState>()
    .add_sub_state::<PauseState>()
    .add_plugins(GameDataPlugin {
        loading: AppState::Loading,
        ready: AppState::Menu,
    })
    .add_plugins(locale::LocalePlugin)
    .add_plugins(fonts::FontsPlugin)
    .add_plugins(avian3d::PhysicsPlugins::default())
    .add_plugins(bevy_hanabi::HanabiPlugin)
    .insert_resource(avian3d::prelude::Gravity(Vec3::ZERO))
    // Single substep: the bunker has no fast-moving rigid bodies
    // (only a player capsule vs static walls). 6 substeps (the
    // default) wastes ~4ms/frame on redundant constraint solving.
    .insert_resource(avian3d::prelude::SubstepCount(1))
    .add_plugins(CordonSimPlugin)
    .add_plugins(lifecycle::LifecyclePlugin)
    .add_plugins(menu::MenuPlugin)
    .add_plugins(bunker::BunkerPlugin)
    .add_plugins(laptop::LaptopPlugin)
    .add_plugins(quest::QuestBridgePlugin);

    #[cfg(feature = "steam")]
    app.add_plugins(steam::SteamPlugin);

    // Dev-time overlays — compiled out of release builds entirely,
    // and only compiled when at least one of the `diagnostic`,
    // `inspector`, or `cheat` features is on. Each feature
    // independently adds its own sub-plugin.
    #[cfg(all(debug_assertions, feature = "development"))]
    app.add_plugins(dev::DevPlugin);

    app.run();
}
