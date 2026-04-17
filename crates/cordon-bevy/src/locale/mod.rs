//! Fluent localization loading and resolution.

use bevy::asset::LoadedFolder;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_fluent::prelude::*;
use fluent_content::Content;

use crate::AppState;

pub struct LocalePlugin;

impl Plugin for LocalePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FluentPlugin);
        app.insert_resource(Locale::new("en-US".parse().expect("valid locale")));
        app.add_systems(Startup, start_locale_load);
        app.add_systems(
            Update,
            build_localization
                .run_if(in_state(AppState::Loading))
                .run_if(resource_exists::<LocaleHandle>)
                .run_if(not(resource_exists::<Localization>)),
        );
    }
}

/// Localization lookup. Use this as a system parameter instead
/// of `Option<Res<Localization>>`. Returns the key itself when
/// no translation is found or localization hasn't loaded yet.
#[derive(SystemParam)]
pub struct L10n<'w> {
    inner: Option<Res<'w, Localization>>,
}

impl L10n<'_> {
    /// Resolve a Fluent key. Falls back to the raw key string.
    pub fn get(&self, key: &str) -> String {
        self.inner
            .as_ref()
            .and_then(|l| l.content(key))
            .unwrap_or_else(|| key.to_string())
    }
}

#[derive(Resource)]
struct LocaleHandle(Handle<LoadedFolder>);

fn start_locale_load(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LocaleHandle(server.load_folder("locale")));
}

fn build_localization(
    handle: Res<LocaleHandle>,
    server: Res<AssetServer>,
    builder: LocalizationBuilder,
    mut commands: Commands,
) {
    if !server.is_loaded_with_dependencies(&handle.0) {
        return;
    }
    let l10n = builder.build(&handle.0);
    info!("Localization built: {:?}", l10n);
    commands.insert_resource(l10n);
}
