//! Fluent localization loading and resolution.

use bevy::asset::LoadedFolder;
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
        // `Localization` is already a `Resource` (derived by
        // `bevy_fluent`), so we insert it directly — no newtype
        // wrapper. Callers read `Option<Res<Localization>>`.
        app.add_systems(
            Update,
            build_localization
                .run_if(in_state(AppState::Loading))
                .run_if(resource_exists::<LocaleHandle>)
                .run_if(not(resource_exists::<Localization>)),
        );
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

/// Resolve a fluent key or return the fallback.
pub fn l10n_or(l10n: &Localization, key: &str, fallback: &str) -> String {
    l10n.content(key).unwrap_or_else(|| fallback.to_string())
}
