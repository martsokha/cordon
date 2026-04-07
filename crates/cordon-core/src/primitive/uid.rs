//! Typed auto-incrementing numeric IDs for runtime entities.
//!
//! [`Uid<T>`] is a numeric identifier parameterized by a marker type,
//! so the compiler prevents mixing up NPC UIDs with mission UIDs, etc.

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A numeric runtime identifier for spawned entities.
///
/// Parameterized by a marker type `T` so the compiler prevents
/// accidentally passing an NPC UID where a mission UID is expected.
/// UIDs are unique within a single game session.
#[derive(Component, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uid<T: Send + Sync + 'static>(u32, #[serde(skip)] PhantomData<fn() -> T>);

impl<T: Send + Sync + 'static> Uid<T> {
    /// Create a new UID from a raw value.
    pub fn new(value: u32) -> Self {
        Self(value, PhantomData)
    }

    /// Get the raw numeric value.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl<T: Send + Sync + 'static> Clone for Uid<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for Uid<T> {}

impl<T: Send + Sync + 'static> Default for Uid<T> {
    fn default() -> Self {
        Self(0, PhantomData)
    }
}

impl<T: Send + Sync + 'static> PartialEq for Uid<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Send + Sync + 'static> Eq for Uid<T> {}

impl<T: Send + Sync + 'static> Hash for Uid<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Send + Sync + 'static> fmt::Debug for Uid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Uid({})", self.0)
    }
}

impl<T: Send + Sync + 'static> fmt::Display for Uid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
