//! Concrete item instances in the game world.

use serde::{Deserialize, Serialize};

use super::ItemData;
use super::data::RelicStability;
use super::def::ItemDef;
use crate::primitive::condition::Condition;
use crate::primitive::id::{Id, Item as ItemMarker};

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
/// [`ItemDef`] by [`Id<Item>`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// ID of the [`ItemDef`] this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Physical condition of this item.
    pub condition: Condition,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
    /// Relic stability state. Only set for relic items.
    pub relic_stability: Option<RelicStability>,
    /// Days remaining until spoilage. `None` means it doesn't spoil.
    pub freshness: Option<u32>,
}

impl Item {
    /// Create a new genuine item with the given condition.
    pub fn new(def_id: Id<ItemMarker>, condition: Condition) -> Self {
        Self {
            def_id,
            condition,
            authenticity: Authenticity::Genuine,
            relic_stability: None,
            freshness: None,
        }
    }

    /// Create a new item from its definition, initializing freshness
    /// and relic stability from the def's type-specific data.
    pub fn from_def(def: &ItemDef, condition: Condition) -> Self {
        let mut item = Self::new(def.id.clone(), condition);

        match &def.data {
            ItemData::Consumable { spoil_days, .. } => {
                item.freshness = *spoil_days;
            }
            ItemData::Relic {
                default_stability, ..
            } => {
                item.relic_stability = Some(*default_stability);
            }
            _ => {}
        }

        item
    }
}
