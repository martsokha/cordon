#![forbid(unsafe_code)]

mod laptop;
mod world;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use cordon_data::gamedata::GameDataPlugin;

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}

fn main() {
    App::new()
        .add_plugins(
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
        .add_plugins(GameDataPlugin {
            loading: AppState::Loading,
            ready: AppState::InGame,
        })
        .add_plugins(world::WorldPlugin)
        .add_plugins(laptop::LaptopPlugin)
        .run();
}
