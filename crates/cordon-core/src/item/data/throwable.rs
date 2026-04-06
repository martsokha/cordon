//! Throwable item data (grenades, molotovs, smoke).

use serde::{Deserialize, Serialize};

use crate::item::effect::Effect;
use crate::primitive::Duration;

/// Data for throwable items.
///
/// Each effect carries its own duration and optional aoe radius.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowableData {
    /// Effects applied on impact. Each has its own duration and aoe.
    pub effects: Vec<Effect>,
    /// Seconds to prime and throw (animation/use time).
    pub use_time: Duration,
}
