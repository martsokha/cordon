//! Consumable item data (food, medicine, drinks, pills).

use serde::{Deserialize, Serialize};

use crate::item::effect::Effect;
use crate::primitive::duration::Duration;

/// Data for consumable items.
///
/// Each effect carries its own duration. A medkit might have an
/// instant heal effect and a 10-second anti-bleeding effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumableData {
    /// Effects applied when consumed. Each has its own duration.
    pub effects: Vec<Effect>,
    /// Seconds to consume this item (animation/use time).
    pub use_time: Duration,
    /// Days until spoilage. `None` means it never spoils.
    pub spoil_days: Option<u32>,
}
