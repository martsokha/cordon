//! Consumable item data (food, medicine, drinks, pills).

use serde::{Deserialize, Serialize};

use crate::item::effect::TimedEffect;

/// Data for consumable items.
///
/// Each effect carries its own duration. A medkit might have an
/// instant heal effect and a timed anti-bleeding effect.
///
/// There is no `use_time` field — every consumable applies on the
/// minute it's used. The sim's minute granularity is coarse enough
/// that "this takes 2 minutes to apply" wouldn't be observable
/// against the surrounding noise of combat ticks anyway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effects applied when consumed. Each has its own duration.
    pub effects: Vec<TimedEffect>,
    /// Days until spoilage. `None` means it never spoils.
    pub spoil_days: Option<u32>,
}
