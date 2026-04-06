//! NPC name generation and localization.
//!
//! Name pools are loaded from config and define which localization
//! keys are available for NPC name generation. At generation time,
//! keys are picked from the pool and stored on the NPC. The UI
//! layer resolves them through fluent at display time.

use serde::{Deserialize, Serialize};

use crate::primitive::id::{Id, IdMarker};

/// Marker for name pool IDs.
pub struct NamePoolMarker;
impl IdMarker for NamePoolMarker {}

/// How names are constructed from the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NameFormat {
    /// Pick a single alias (e.g., "Viper", "Ghost").
    /// Used by Drifters, Syndicate, Mercenaries.
    Alias,
    /// Combine a first name and surname (e.g., "Sergei Volkov").
    /// Used by Garrison, Institute.
    FirstSurname,
    /// Combine a first name and alias (e.g., "Sergei 'Viper'").
    FirstAlias,
}

/// A pool of name keys for NPC generation, loaded from config.
///
/// Each entry is a fluent localization key (e.g., `"name-sergei"`).
/// The UI resolves these keys to display strings at render time,
/// so names adapt to the active locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePool {
    /// Unique identifier (e.g., `"slavic"`, `"western"`).
    pub id: Id<NamePoolMarker>,
    /// How names are assembled from this pool's lists.
    pub format: NameFormat,
    /// First name keys. Always used.
    pub names: Vec<String>,
    /// Surname keys. Used with [`NameFormat::FirstSurname`].
    pub surnames: Vec<String>,
    /// Alias keys. Used with [`NameFormat::Alias`] and [`NameFormat::FirstAlias`].
    pub aliases: Vec<String>,
}

/// An NPC's name, stored as localization keys.
///
/// Resolved to a display string by the UI layer through fluent.
/// The format determines how the parts are combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcName {
    /// How this name should be formatted.
    pub format: NameFormat,
    /// Primary key: a first name or alias key.
    pub first: String,
    /// Secondary key: a surname or alias key.
    /// `None` for [`NameFormat::Alias`].
    pub second: Option<String>,
}
