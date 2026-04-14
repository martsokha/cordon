//! Fluent localization loading and resolution.
//!
//! Also owns the cross-resource validation pass that warns when
//! content data (NPC templates, eventually items/factions/areas)
//! references a fluent key that's not present in the loaded
//! locale. That check lives here rather than in the cordon-data
//! catalog validator because the locale only becomes available
//! on the Bevy side, after asset loading.

use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_data::gamedata::GameDataResource;
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
        // Cross-resource validation: runs once both the catalog
        // and the locale are live. Gated by `L10nValidated` so it
        // fires exactly once per game session, not every frame.
        app.add_systems(
            Update,
            validate_content_keys
                .run_if(resource_exists::<GameDataResource>)
                .run_if(resource_exists::<Localization>)
                .run_if(not(resource_exists::<L10nValidated>)),
        );
    }
}

/// Marker inserted after [`validate_content_keys`] runs, so the
/// pass doesn't repeat every frame.
#[derive(Resource)]
struct L10nValidated;

/// Walk every content definition that carries a fluent key and
/// warn on missing entries. Runs once when both the catalog and
/// the locale are loaded.
///
/// Currently covers:
/// - `NpcTemplateDef::name_key`
///
/// Extend here when new content types gain localizable fields.
fn validate_content_keys(
    mut commands: Commands,
    data: Res<GameDataResource>,
    l10n: Res<Localization>,
) {
    let mut missing = 0usize;
    for (id, def) in &data.0.npc_templates {
        if l10n.content(&def.name_key).is_none() {
            warn!(
                "locale: npc template `{}` references missing fluent key `{}`",
                id.as_str(),
                def.name_key,
            );
            missing += 1;
        }
    }
    if missing == 0 {
        info!("locale: content key validation passed");
    } else {
        warn!("locale: {missing} missing fluent key(s) in content");
    }
    commands.insert_resource(L10nValidated);
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
