//! Throwable item data (grenades, molotovs, smoke).

use serde::{Deserialize, Serialize};

use crate::item::effect::TimedEffect;

/// Data for throwable items.
///
/// Each effect carries its own duration and optional aoe radius.
/// Throwables are instant-use at minute granularity — no prime
/// time, no arming delay — so there is no `use_time` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrowableData {
    /// Effects applied on impact. Each has its own duration and aoe.
    pub effects: Vec<TimedEffect>,
}
