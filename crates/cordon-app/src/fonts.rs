//! Shared font resources loaded at startup.

use bevy::prelude::*;

/// Primary UI font, used across laptop and bunker interfaces.
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

pub struct FontsPlugin;

impl Plugin for FontsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load);
    }
}

fn load(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(UiFont(server.load("fonts/PTMono-Regular.ttf")));
}
