//! Concrete item instances in the game world.

use serde::{Deserialize, Serialize};

use crate::primitive::condition::Condition;
use crate::primitive::id::Id;

/// Whether an item is genuine or has been tampered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Authenticity {
    /// Real, unmodified item.
    Genuine,
    /// Fake: looks real but has no properties (or harmful ones).
    Counterfeit,
    /// Past its effective date (meds, food).
    Expired,
    /// Planted by a faction to mislead (documents).
    Doctored,
}

/// A single item instance in the game world.
///
/// Items do not stack — each instance is tracked individually with
/// its own condition, authenticity, and freshness. References an
/// [`ItemDef`](super::ItemDef) by [`Id`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// ID of the [`ItemDef`](super::ItemDef) this is an instance of.
    pub def_id: Id,
    /// Physical condition of this item.
    pub condition: Condition,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
    /// Days remaining until spoilage. `None` means it doesn't spoil.
    /// Initialized from [`ItemData::Consumable::spoil_days`](super::ItemData::Consumable)
    /// when created.
    pub freshness: Option<u32>,
}

impl Item {
    /// Create a new genuine item with the given condition.
    pub fn new(def_id: Id, condition: Condition) -> Self {
        Self {
            def_id,
            condition,
            authenticity: Authenticity::Genuine,
            freshness: None,
        }
    }
}
