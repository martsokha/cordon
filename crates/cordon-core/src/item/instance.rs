//! Concrete item instances in the game world.

use serde::{Deserialize, Serialize};

use crate::item::def::Item as ItemMarker;
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
/// its own condition and authenticity. References an [`ItemDef`](super::ItemDef)
/// by [`Id`].
///
/// Condition represents both physical wear (weapons, armor) and
/// freshness (food, meds). A consumable at condition 0.0 has spoiled.
/// Relic stability is defined by the relic type in [`RelicData`](super::RelicData),
/// not per-instance — it's an inherent property of that relic kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// ID of the [`ItemDef`](super::ItemDef) this is an instance of.
    pub def_id: Id<ItemMarker>,
    /// Condition from 0.0 (destroyed/spoiled) to 1.0 (factory new/fresh).
    /// For weapons/armor: physical wear. For consumables: freshness.
    /// Price scales with condition².
    pub condition: Condition,
    /// Whether this item is genuine, counterfeit, expired, or doctored.
    pub authenticity: Authenticity,
}

impl Item {
    /// Create a new genuine item with the given condition.
    pub fn new(def_id: Id<ItemMarker>, condition: Condition) -> Self {
        Self {
            def_id,
            condition,
            authenticity: Authenticity::Genuine,
        }
    }
}
