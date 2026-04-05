//! Identifiers used throughout the game.
//!
//! [`Id`] is used for data-driven objects defined in config files (factions,
//! items, sectors, etc.). [`Uid`] is used for runtime-spawned entities
//! (NPCs, missions) that need a unique handle within a game session.

use std::fmt;

use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

/// A string-based identifier for data-driven game objects.
///
/// Factions, calibers, sectors, perks, upgrades, items, etc. are all
/// defined in JSON config files and referenced by their string ID.
///
/// IDs are case-sensitive, lowercase, snake_case by convention
/// (e.g., `"order"`, `"9x18mm"`, `"threshold"`, `"scavengers_eye"`).
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, From)]
#[display("{_0}")]
pub struct Id(String);

impl Id {
    /// Create a new ID from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id(\"{}\")", self.0)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Auto-incrementing numeric ID for runtime-spawned entities.
///
/// Used for NPCs and missions that are created during gameplay,
/// not loaded from config. Each [`Uid`] is unique within a single
/// game session.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    From
)]
#[display("{_0}")]
pub struct Uid(pub u32);
