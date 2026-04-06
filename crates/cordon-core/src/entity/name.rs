//! NPC name generation pools.
//!
//! Name pools are loaded from config and define how NPC names are
//! generated for each faction. Some factions use aliases (callsigns),
//! others use first + surname or first + alias combinations.

use serde::{Deserialize, Serialize};

use crate::primitive::id::Id;

/// How names are constructed from the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NameFormat {
    /// Pick a single alias from the pool (e.g., "Viper", "Ghost").
    /// Used by Drifters, Syndicate, Mercenaries.
    Alias,
    /// Combine a first name and surname (e.g., "Sergei Volkov").
    /// Used by Garrison, Institute.
    FirstSurname,
    /// Combine a first name and alias (e.g., "Sergei 'Viper'").
    FirstAlias,
}

/// A pool of names for NPC generation, loaded from config.
///
/// Each faction references a name pool by ID. Multiple factions
/// can share a pool. The [`id`](NamePool::id) doubles as the
/// localization key prefix for the pool's metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePool {
    /// Unique identifier (e.g., `"drifter_aliases"`, `"garrison_names"`).
    pub id: Id<NamePoolMarker>,
    /// How names are assembled from this pool's lists.
    pub format: NameFormat,
    /// First names or aliases. Always used.
    pub names: Vec<String>,
    /// Surnames. Used with [`NameFormat::FirstSurname`].
    pub surnames: Vec<String>,
    /// Aliases used as second part in [`NameFormat::FirstAlias`].
    pub aliases: Vec<String>,
}

/// Marker for name pool IDs.
pub struct NamePoolMarker;
impl crate::primitive::id::IdMarker for NamePoolMarker {}
