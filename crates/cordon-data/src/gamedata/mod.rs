//! Bevy plugin for loading game data from JSON assets.
//!
//! All game data lives in `assets/data/` as subdirectories of JSON
//! files. Each subdirectory is loaded as a folder, deserialized into
//! the correct type, and assembled into a [`GameData`] catalog.
//!
//! ```text
//! assets/data/
//!   areas/          → Vec<AreaDef>
//!   events/         → Vec<EventDef>
//!   factions/       → Vec<FactionDef>
//!   items/          → Vec<ItemDef>
//!   namepools/      → Vec<NamePool>
//!   perks/          → Vec<PerkDef>
//!   quests/         → Vec<QuestDef>
//!   upgrades/       → Vec<UpgradeDef>
//! ```

use std::collections::HashMap;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadedFolder};
use bevy::prelude::*;
use bevy::state::state::FreelyMutableState;
use cordon_core::entity::bunker::UpgradeDef;
use cordon_core::entity::faction::FactionDef;
use cordon_core::entity::name::NamePool;
use cordon_core::entity::perk::PerkDef;
use cordon_core::item::ItemDef;
use cordon_core::primitive::Id;
use cordon_core::world::area::AreaDef;
use cordon_core::world::event::EventDef;
use cordon_core::world::loot::LootTables;
use cordon_core::world::narrative::quest::QuestDef;

use crate::catalog::GameData;

/// A raw JSON asset — holds unparsed bytes from a `.json` file.
#[derive(Asset, TypePath)]
pub struct RawJson(pub Vec<u8>);

/// Loader for raw JSON files.
#[derive(Default, TypePath)]
struct RawJsonLoader;

#[derive(Debug, thiserror::Error)]
#[error("io error: {0}")]
struct RawJsonError(#[from] std::io::Error);

impl AssetLoader for RawJsonLoader {
    type Asset = RawJson;
    type Error = RawJsonError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _ctx: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(RawJson(bytes))
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// All folder handles being loaded.
#[derive(Resource)]
struct LoadingFolders {
    areas: Handle<LoadedFolder>,
    events: Handle<LoadedFolder>,
    factions: Handle<LoadedFolder>,
    items: Handle<LoadedFolder>,
    namepools: Handle<LoadedFolder>,
    perks: Handle<LoadedFolder>,
    quests: Handle<LoadedFolder>,
    upgrades: Handle<LoadedFolder>,
}

impl LoadingFolders {
    fn all_loaded(&self, server: &AssetServer) -> bool {
        server.is_loaded_with_dependencies(&self.areas)
            && server.is_loaded_with_dependencies(&self.events)
            && server.is_loaded_with_dependencies(&self.factions)
            && server.is_loaded_with_dependencies(&self.items)
            && server.is_loaded_with_dependencies(&self.namepools)
            && server.is_loaded_with_dependencies(&self.perks)
            && server.is_loaded_with_dependencies(&self.quests)
            && server.is_loaded_with_dependencies(&self.upgrades)
    }
}

/// Bevy plugin that loads all game data from `assets/data/`.
///
/// Generic over the app's state type. Provide the loading state
/// (when to start loading) and the target state (when loading is done).
pub struct GameDataPlugin<S: FreelyMutableState> {
    /// State during which loading happens.
    pub loading: S,
    /// State to transition to when all data is loaded.
    pub ready: S,
}

impl<S: FreelyMutableState> Plugin for GameDataPlugin<S> {
    fn build(&self, app: &mut App) {
        let loading = self.loading.clone();
        let ready = self.ready.clone();

        app.init_asset::<RawJson>()
            .init_asset_loader::<RawJsonLoader>()
            .add_systems(OnEnter(loading.clone()), start_loading)
            .add_systems(
                Update,
                assemble_game_data::<S>
                    .run_if(in_state(loading))
                    .run_if(resource_exists::<LoadingFolders>),
            )
            .insert_resource(ReadyState(ready));
    }
}

/// Holds the target state to transition to after loading.
#[derive(Resource)]
struct ReadyState<S: FreelyMutableState>(S);

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LoadingFolders {
        areas: server.load_folder("data/areas"),
        events: server.load_folder("data/events"),
        factions: server.load_folder("data/factions"),
        items: server.load_folder("data/items"),
        namepools: server.load_folder("data/namepools"),
        perks: server.load_folder("data/perks"),
        quests: server.load_folder("data/quests"),
        upgrades: server.load_folder("data/upgrades"),
    });
}

fn assemble_game_data<S: FreelyMutableState>(
    loading: Res<LoadingFolders>,
    server: Res<AssetServer>,
    folders: Res<Assets<LoadedFolder>>,
    raw: Res<Assets<RawJson>>,
    ready: Res<ReadyState<S>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<S>>,
) {
    if !loading.all_loaded(&server) {
        return;
    }

    let areas = parse_folder(&loading.areas, &folders, &raw, |d: &AreaDef| d.id.clone());
    let events = parse_folder(&loading.events, &folders, &raw, |d: &EventDef| d.id.clone());
    let factions = parse_folder(&loading.factions, &folders, &raw, |d: &FactionDef| {
        d.id.clone()
    });
    let items = parse_folder(&loading.items, &folders, &raw, |d: &ItemDef| d.id.clone());
    let name_pools = parse_folder(&loading.namepools, &folders, &raw, |d: &NamePool| {
        d.id.clone()
    });
    let perks = parse_folder(&loading.perks, &folders, &raw, |d: &PerkDef| d.id.clone());
    let quests = parse_folder(&loading.quests, &folders, &raw, |d: &QuestDef| d.id.clone());
    let upgrades = parse_folder(&loading.upgrades, &folders, &raw, |d: &UpgradeDef| {
        d.id.clone()
    });

    info!(
        "Game data loaded: {} areas, {} events, {} factions, {} items, {} namepools, {} perks, {} quests, {} upgrades",
        areas.len(),
        events.len(),
        factions.len(),
        items.len(),
        name_pools.len(),
        perks.len(),
        quests.len(),
        upgrades.len(),
    );

    commands.insert_resource(GameDataResource(GameData {
        areas,
        events,
        factions,
        items,
        name_pools,
        perks,
        quests,
        upgrades,
        loot_tables: LootTables::default(),
    }));
    commands.remove_resource::<LoadingFolders>();
    commands.remove_resource::<ReadyState<S>>();
    *next_state = NextState::Pending(ready.0.clone());
}

/// Parse all JSON files in a loaded folder into a keyed HashMap.
fn parse_folder<T, M>(
    handle: &Handle<LoadedFolder>,
    folders: &Assets<LoadedFolder>,
    raw: &Assets<RawJson>,
    key: impl Fn(&T) -> Id<M>,
) -> HashMap<Id<M>, T>
where
    T: serde::de::DeserializeOwned + Clone,
    M: cordon_core::primitive::IdMarker,
{
    let mut map = HashMap::new();
    let Some(folder) = folders.get(handle) else {
        return map;
    };
    for file_handle in &folder.handles {
        let Some(json) = raw.get(&file_handle.clone().typed::<RawJson>()) else {
            continue;
        };
        match serde_json::from_slice::<Vec<T>>(&json.0) {
            Ok(defs) => {
                for def in defs {
                    map.insert(key(&def), def);
                }
            }
            Err(e) => {
                warn!("Failed to parse JSON: {e}");
            }
        }
    }
    map
}

/// The assembled game data, available as a Bevy resource after loading.
#[derive(Resource)]
pub struct GameDataResource(pub GameData);
