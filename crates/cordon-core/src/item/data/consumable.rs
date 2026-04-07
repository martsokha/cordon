//! Consumable item data (food, medicine, drinks, pills).

use serde::{Deserialize, Serialize};

use crate::item::effect::TimedEffect;
use crate::primitive::Duration;

/// Data for consumable items.
///
/// Each effect carries its own duration. A medkit might have an
/// instant heal effect and a timed anti-bleeding effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effects applied when consumed. Each has its own duration.
    pub effects: Vec<TimedEffect>,
    /// Seconds to consume this item (animation/use time).
    pub use_time: Duration,
    /// Days until spoilage. `None` means it never spoils.
    pub spoil_days: Option<u32>,
}
