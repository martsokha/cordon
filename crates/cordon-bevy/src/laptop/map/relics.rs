//! Relic map visuals: shared mesh + material, preloaded icon
//! cache, and the attach-on-spawn system that gives every relic
//! its map dot.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use super::MapWorldEntity;
use crate::PlayingState;

const COLOR_RELIC: Color = Color::srgb(0.3, 0.9, 1.0);

/// Shared mesh + material handles for world relics.
#[derive(Resource, Clone)]
pub struct RelicAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

/// Preloaded relic icon `Handle<Image>`s, keyed by item id. Filled
/// once on laptop entry by [`preload_relic_icons`] so the hover
/// system can hand the renderer a resolved handle without calling
/// `asset_server.load()` or formatting a path string on every tick.
#[derive(Resource, Default)]
pub struct RelicIconAssets {
    handles: HashMap<cordon_core::primitive::Id<cordon_core::item::Item>, Handle<Image>>,
}

impl RelicIconAssets {
    pub fn get(
        &self,
        id: &cordon_core::primitive::Id<cordon_core::item::Item>,
    ) -> Option<Handle<Image>> {
        self.handles.get(id).cloned()
    }
}

pub struct RelicsPlugin;

impl Plugin for RelicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_relic_assets);
        app.add_systems(
            OnEnter(PlayingState::Laptop),
            preload_relic_icons.run_if(not(resource_exists::<RelicIconAssets>)),
        );
        app.add_systems(
            Update,
            attach_relic_visuals
                .after(cordon_sim::plugin::SimSet::Spawn)
                .run_if(in_state(crate::AppState::Playing)),
        );
    }
}

fn init_relic_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Circle::new(4.0));
    let material = materials.add(ColorMaterial::from_color(COLOR_RELIC));
    commands.insert_resource(RelicAssets { mesh, material });
}

/// Walk every relic `ItemDef` in the catalog and preload its icon
/// into a `Handle<Image>`, stored by item id in [`RelicIconAssets`].
///
/// Runs once on [`PlayingState::Laptop`] entry because that's the
/// first time both (a) the laptop UI is visible and (b) game data
/// is guaranteed loaded. 36 small PNGs is negligible to hold
/// resident.
fn preload_relic_icons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_data: Res<GameDataResource>,
) {
    let mut handles = HashMap::new();
    for (id, def) in &game_data.0.items {
        if matches!(def.data, cordon_core::item::ItemData::Relic(_)) {
            // Icon filenames are intentionally *not* prefixed with
            // `item_` — they live in `icons/relics/<short>.png`
            // where `<short>` is the id with the `item_` prefix
            // stripped. Filesystem paths don't need the prefix for
            // disambiguation (the directory scopes them) and not
            // renaming every PNG file is the right trade.
            let short = id.as_str().strip_prefix("item_").unwrap_or(id.as_str());
            let path = format!("icons/relics/{short}.png");
            handles.insert(id.clone(), asset_server.load(path));
        }
    }
    commands.insert_resource(RelicIconAssets { handles });
}

/// Attach map visuals to newly-spawned relic entities. The
/// tooltip payload is built on-demand by the hover system from
/// the relic's catalog def, so no per-entity info component is
/// needed here.
fn attach_relic_visuals(
    assets: Res<RelicAssets>,
    new_relics: Query<Entity, Added<cordon_sim::components::RelicMarker>>,
    mut commands: Commands,
) {
    if new_relics.iter().next().is_none() {
        return;
    }
    for entity in &new_relics {
        commands.entity(entity).insert((
            MapWorldEntity,
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(assets.material.clone()),
        ));
    }
}
