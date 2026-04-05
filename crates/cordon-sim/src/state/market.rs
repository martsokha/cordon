//! Live market state tracking supply and demand fluctuations.

use std::collections::HashMap;

use cordon_core::primitive::condition::Condition;
use cordon_core::primitive::id::Id;
use cordon_core::world::price::PriceModifiers;

/// Live market state for the current game session.
///
/// Tracks per-item supply and demand levels plus a global event modifier.
/// Used to compute final prices for buying and selling.
pub struct MarketState {
    /// Supply levels per item ID (1.0 = normal).
    pub supply: HashMap<Id, f32>,
    /// Demand levels per item ID (1.0 = normal).
    pub demand: HashMap<Id, f32>,
    /// Global event modifier (affects all prices).
    pub event_modifier: f32,
}

impl MarketState {
    /// Create a new market state with neutral supply/demand.
    pub fn new() -> Self {
        Self {
            supply: HashMap::new(),
            demand: HashMap::new(),
            event_modifier: 1.0,
        }
    }

    /// Build [`PriceModifiers`] for a specific item from current market conditions.
    pub fn get_modifiers(
        &self,
        item_id: &Id,
        faction_modifier: f32,
        reputation: f32,
    ) -> PriceModifiers {
        PriceModifiers {
            supply: self.supply.get(item_id).copied().unwrap_or(1.0),
            demand: self.demand.get(item_id).copied().unwrap_or(1.0),
            faction: faction_modifier,
            event: self.event_modifier,
            reputation,
        }
    }

    /// Compute the final price for an item given its base price and condition.
    pub fn get_price(
        &self,
        base_price: u32,
        condition: Condition,
        item_id: &Id,
        faction_modifier: f32,
        reputation: f32,
    ) -> u32 {
        let mods = self.get_modifiers(item_id, faction_modifier, reputation);
        mods.final_price(base_price, condition)
    }
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new()
    }
}
