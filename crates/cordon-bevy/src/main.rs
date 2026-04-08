#![forbid(unsafe_code)]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod bunker;
#[cfg(debug_assertions)]
mod debug;
mod laptop;
mod locale;
mod quest;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use cordon_data::gamedata::GameDataPlugin;
use cordon_sim::plugin::CordonSimPlugin;
use cordon_sim::resources::init_world_resources;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Loading,
    Playing,
}

#[derive(SubStates, Default, Clone, Eq, PartialEq, Hash, Debug)]
#[source(AppState = AppState::Playing)]
pub enum PlayingState {
    #[default]
    Bunker,
    Laptop,
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
    .add_plugins(GameDataPlugin {
        loading: AppState::Loading,
        ready: AppState::Playing,
    })
    .add_plugins(locale::LocalePlugin)
    .add_plugins(CordonSimPlugin)
    .add_plugins(bunker::BunkerPlugin)
    .add_plugins(laptop::LaptopPlugin)
    .add_plugins(quest::QuestBridgePlugin);

    // Bootstrap the cordon-sim resource set on enter-play.
    // `init_world_resources` lives in cordon-sim — it knows how to
    // read `GameDataResource` and populate the world. The hook is
    // here in cordon-bevy because `AppState` is a bevy-layer type.
    app.add_systems(OnEnter(AppState::Playing), init_world_resources);

    // Debug overlay + world inspector + dev cheats — compiled out
    // of release builds entirely via the `#[cfg(debug_assertions)]`
    // on the `mod debug;` declaration above.
    #[cfg(debug_assertions)]
    app.add_plugins(debug::DebugPlugin);

    app.run();
}
