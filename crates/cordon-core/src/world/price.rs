//! Price calculation with exponential condition scaling.
//!
//! The final price of an item is its base price multiplied by condition²
//! and several world-state modifiers. This makes low-condition gear
//! nearly worthless and creates a repair arbitrage opportunity.

use crate::primitive::condition::Condition;

/// Modifiers applied to an item's base price from the world state.
///
/// Each modifier is a multiplier around 1.0. Values below 1.0 reduce
/// the price, values above 1.0 increase it.
#[derive(Debug, Clone, Copy)]
pub struct PriceModifiers {
    /// Supply level for this item type (0.5–1.5). High supply = lower price.
    pub supply: f32,
    /// Demand level for this item type (0.5–2.0). High demand = higher price.
    pub demand: f32,
    /// Faction control modifier (0.8–1.2). Dominant faction's gear is cheaper.
    pub faction: f32,
    /// Active event modifier (0.5–3.0). Surges, shortages, etc. swing prices.
    pub event: f32,
    /// Player reputation modifier (0.9–1.1). Trusted traders get better margins.
    pub reputation: f32,
}

impl PriceModifiers {
    /// Compute the final price for an item.
    ///
    /// Uses [`Condition::price_factor`] (condition²) so a 0.5 item is
    /// worth ~25% of base, 0.75 ≈ 56%. The result is always at least
    /// 1 credit.
    pub fn final_price(&self, base_price: u32, condition: Condition) -> u32 {
        let price = base_price as f64
            * condition.price_factor() as f64
            * self.supply as f64
            * self.demand as f64
            * self.faction as f64
            * self.event as f64
            * self.reputation as f64;

        (price.round() as u32).max(1)
    }
}

impl Default for PriceModifiers {
    fn default() -> Self {
        Self {
            supply: 1.0,
            demand: 1.0,
            faction: 1.0,
            event: 1.0,
            reputation: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_squared_pricing() {
        let mods = PriceModifiers::default();
        let base = 10000;

        assert_eq!(mods.final_price(base, Condition::PERFECT), 10000);
        assert_eq!(mods.final_price(base, Condition::new(0.75)), 5625);
        assert_eq!(mods.final_price(base, Condition::new(0.5)), 2500);
        assert_eq!(mods.final_price(base, Condition::new(0.25)), 625);
    }

    #[test]
    fn price_never_zero() {
        let mods = PriceModifiers::default();
        assert_eq!(mods.final_price(100, Condition::new(0.01)), 1);
    }
}
