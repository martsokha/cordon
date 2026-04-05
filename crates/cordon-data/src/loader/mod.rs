//! Asset loading from the filesystem.
//!
//! The [`Loader`] reads JSON files from an asset directory and
//! assembles a [`GameData`] catalog. The expected directory layout:
//!
//! ```text
//! assets/
//!   data/
//!     items/
//!       weapons.json      (Vec<ItemDef>)
//!       ammo.json
//!       armor.json
//!       consumables.json
//!       relics.json
//!       documents.json
//!       tech.json
//!       throwables.json
//!       attachments.json
//!     factions.json       (Vec<FactionDef>)
//!     areas.json          (Vec<AreaDef>)
//!     events.json         (Vec<EventDef>)
//!     upgrades.json       (Vec<UpgradeDef>)
//!     perks.json          (Vec<PerkDef>)
//!     name_pools.json     (Vec<NamePool>)
//!     loot_tables.json    (map of area ID -> LootTable)
//!     quests/
//!       *.json            (Vec<QuestDef>, one file per quest line)
//!   locale/
//!     en/
//!       ...
//! ```
//!
//! Each JSON file contains a list of definitions. The loader reads all
//! files, deserializes them, and inserts into the appropriate HashMap
//! in [`GameData`].

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use cordon_core::bunker::UpgradeDef;
use cordon_core::entity::faction::FactionDef;
use cordon_core::entity::name::{NamePool, NamePoolMarker};
use cordon_core::entity::npc::PerkDef;
use cordon_core::item::ItemDef;
use cordon_core::primitive::id::{Area, Event, Faction, Id, Item, Perk, Quest, Upgrade};
use cordon_core::world::area::AreaDef;
use cordon_core::world::event::EventDef;
use cordon_core::world::quest::QuestDef;

use crate::catalog::GameData;
use crate::loot::LootTables;

/// Errors that can occur during asset loading.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Failed to read a file from disk.
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to parse JSON.
    #[error("failed to parse {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// A required directory is missing.
    #[error("missing directory: {0}")]
    MissingDir(PathBuf),
}

/// Loads game data from an asset directory.
///
/// Create a loader with the path to the `assets/data/` directory,
/// then call [`load`](Loader::load) to build a [`GameData`] catalog.
pub struct Loader {
    base: PathBuf,
}

impl Loader {
    /// Create a new loader pointing at the given data directory.
    ///
    /// The path should point to the `assets/data/` directory
    /// (containing `items/`, `factions.json`, etc.).
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// Load all game data from the asset directory.
    pub fn load(&self) -> Result<GameData, LoadError> {
        let items = self.load_items()?;
        let factions = self.load_vec::<FactionDef, Faction>("factions.json")?;
        let areas = self.load_vec::<AreaDef, Area>("areas.json")?;
        let events = self.load_vec::<EventDef, Event>("events.json")?;
        let upgrades = self.load_vec::<UpgradeDef, Upgrade>("upgrades.json")?;
        let perks = self.load_vec::<PerkDef, Perk>("perks.json")?;
        let name_pools = self.load_name_pools()?;
        let quests = self.load_quests()?;
        let loot_tables = self.load_loot_tables()?;

        Ok(GameData {
            items,
            factions,
            areas,
            perks,
            upgrades,
            events,
            quests,
            name_pools,
            loot_tables,
        })
    }

    /// Load items from all JSON files in the `items/` subdirectory.
    fn load_items(&self) -> Result<HashMap<Id<Item>, ItemDef>, LoadError> {
        let dir = self.base.join("items");
        if !dir.is_dir() {
            return Ok(HashMap::new());
        }

        let mut map = HashMap::new();
        for entry in self.read_dir(&dir)? {
            let defs: Vec<ItemDef> = self.read_json(&entry)?;
            for def in defs {
                map.insert(def.id.clone(), def);
            }
        }
        Ok(map)
    }

    /// Load quests from all JSON files in the `quests/` subdirectory.
    fn load_quests(&self) -> Result<HashMap<Id<Quest>, QuestDef>, LoadError> {
        let dir = self.base.join("quests");
        if !dir.is_dir() {
            return Ok(HashMap::new());
        }

        let mut map = HashMap::new();
        for entry in self.read_dir(&dir)? {
            let defs: Vec<QuestDef> = self.read_json(&entry)?;
            for def in defs {
                map.insert(def.id.clone(), def);
            }
        }
        Ok(map)
    }

    /// Load name pools from `name_pools.json`.
    fn load_name_pools(&self) -> Result<HashMap<Id<NamePoolMarker>, NamePool>, LoadError> {
        let path = self.base.join("name_pools.json");
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let pools: Vec<NamePool> = self.read_json(&path)?;
        Ok(pools.into_iter().map(|p| (p.id.clone(), p)).collect())
    }

    /// Load loot tables from `loot_tables.json`.
    fn load_loot_tables(&self) -> Result<LootTables, LoadError> {
        let path = self.base.join("loot_tables.json");
        if !path.exists() {
            return Ok(LootTables::default());
        }
        self.read_json(&path)
    }

    /// Load a single JSON file containing a `Vec<T>` and key by ID.
    ///
    /// `T` must have an `id` field. The generic `M` is the ID marker type.
    fn load_vec<T, M>(&self, filename: &str) -> Result<HashMap<Id<M>, T>, LoadError>
    where
        T: serde::de::DeserializeOwned + HasId<M>,
        M: cordon_core::primitive::id::IdMarker,
    {
        let path = self.base.join(filename);
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let defs: Vec<T> = self.read_json(&path)?;
        Ok(defs.into_iter().map(|d| (d.id().clone(), d)).collect())
    }

    /// Read and deserialize a JSON file.
    fn read_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T, LoadError> {
        let contents = fs::read_to_string(path).map_err(|e| LoadError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_str(&contents).map_err(|e| LoadError::Json {
            path: path.to_path_buf(),
            source: e,
        })
    }

    /// List all `.json` files in a directory, sorted for determinism.
    fn read_dir(&self, dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| LoadError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .collect();
        files.sort();
        Ok(files)
    }
}

/// Trait for types that have an ID field. Used by [`Loader::load_vec`]
/// to generically extract the key from a definition.
pub trait HasId<M: cordon_core::primitive::id::IdMarker> {
    /// Get a reference to this definition's ID.
    fn id(&self) -> &Id<M>;
}

impl HasId<Faction> for FactionDef {
    fn id(&self) -> &Id<Faction> {
        &self.id
    }
}

impl HasId<Area> for AreaDef {
    fn id(&self) -> &Id<Area> {
        &self.id
    }
}

impl HasId<Event> for EventDef {
    fn id(&self) -> &Id<Event> {
        &self.id
    }
}

impl HasId<Upgrade> for UpgradeDef {
    fn id(&self) -> &Id<Upgrade> {
        &self.id
    }
}

impl HasId<Perk> for PerkDef {
    fn id(&self) -> &Id<Perk> {
        &self.id
    }
}
