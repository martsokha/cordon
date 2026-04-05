//! Auto-incrementing numeric ID for runtime-spawned entities.

use derive_more::{Display, From};
use serde::{Deserialize, Serialize};

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
