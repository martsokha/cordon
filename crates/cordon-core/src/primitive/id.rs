//! Typed identifiers used throughout the game.
//!
//! [`Id<T>`] is a string-based identifier parameterized by a marker type,
//! so the compiler prevents mixing up faction IDs with item IDs, etc.
//!
//! Use `Id<Faction>`, `Id<Item>`, `Id<Area>`, etc. directly — no type
//! aliases.

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// Marker trait for ID types. Implemented by empty structs that
/// serve as phantom type parameters for [`Id<T>`].
pub trait IdMarker: 'static {}

/// A string-based identifier for data-driven game objects.
///
/// Parameterized by a marker type `T` so the compiler prevents
/// accidentally passing a faction ID where an item ID is expected.
///
/// IDs are case-sensitive, lowercase, snake_case by convention
/// (e.g., `"garrison"`, `"9x18mm"`, `"threshold"`, `"scavengers_eye"`).
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T: IdMarker>(String, #[serde(skip)] PhantomData<T>);

impl<T: IdMarker> Id<T> {
    /// Create a new ID from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into(), PhantomData)
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: IdMarker> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T: IdMarker> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: IdMarker> Eq for Id<T> {}

impl<T: IdMarker> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: IdMarker> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id(\"{}\")", self.0)
    }
}

impl<T: IdMarker> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<T: IdMarker> From<&str> for Id<T> {
    fn from(s: &str) -> Self {
        Self(s.to_string(), PhantomData)
    }
}

impl<T: IdMarker> From<String> for Id<T> {
    fn from(s: String) -> Self {
        Self(s, PhantomData)
    }
}
