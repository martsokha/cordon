//! A duration value in seconds.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// A game duration in seconds.
///
/// Wraps an `Option<NonZeroU32>` — `None` means instant (zero time),
/// `Some(n)` means `n` seconds. The `Option` is the same size as a
/// `u32` thanks to [`NonZeroU32`]'s niche optimization.
///
/// Used for effect durations, consumable use times, throwable prime
/// times, and any other game timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Duration(Option<NonZeroU32>);

impl Duration {
    /// An instant duration (zero seconds).
    pub const INSTANT: Self = Self(None);

    /// Create a duration from a number of seconds. Returns [`INSTANT`](Self::INSTANT) for 0.
    pub fn new(seconds: u32) -> Self {
        Self(NonZeroU32::new(seconds))
    }

    /// Get the raw seconds value. Returns 0 for instant.
    pub fn seconds(self) -> u32 {
        match self.0 {
            Some(n) => n.get(),
            None => 0,
        }
    }

    /// Whether this duration is instant (zero seconds).
    pub fn is_instant(self) -> bool {
        self.0.is_none()
    }
}

impl From<u32> for Duration {
    fn from(seconds: u32) -> Self {
        Self::new(seconds)
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(n) => write!(f, "{}s", n),
            None => write!(f, "instant"),
        }
    }
}
