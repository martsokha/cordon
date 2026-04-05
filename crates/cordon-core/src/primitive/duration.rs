//! A duration value in seconds.

use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

/// A duration in seconds.
///
/// Used for consumable use times, throwable prime times, effect
/// durations, and any other game timing. Wraps a `u32` — zero
/// means instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Display, From)]
#[display("{_0}s")]
pub struct Duration(pub u32);

impl Duration {
    /// An instant duration (0 seconds).
    pub const INSTANT: Self = Self(0);

    /// Get the raw seconds value.
    pub fn seconds(self) -> u32 {
        self.0
    }

    /// Whether this duration is instant (0 seconds).
    pub fn is_instant(self) -> bool {
        self.0 == 0
    }
}
