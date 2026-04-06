#![forbid(unsafe_code)]

mod laptop;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use cordon_data::gamedata::{AppState, GameDataPlugin};

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
        .add_plugins(GameDataPlugin)
        .add_plugins(laptop::LaptopPlugin)
        .run();
}
