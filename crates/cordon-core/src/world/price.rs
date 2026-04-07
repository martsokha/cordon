//! Price calculation with world-state modifiers.
//!
//! The final price of an item is its base price multiplied by several
//! world-state modifiers (supply, demand, faction control, active
//! events, player reputation).

use crate::primitive::Credits;

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
    /// Compute the final price for an item. The result is always at
    /// least 1 credit.
    pub fn final_price(&self, base_price: Credits) -> Credits {
        let price = base_price.value() as f64
            * self.supply as f64
            * self.demand as f64
            * self.faction as f64
            * self.event as f64
            * self.reputation as f64;

        Credits::new((price.round() as u32).max(1))
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
    fn default_modifiers_yield_base_price() {
        let mods = PriceModifiers::default();
        assert_eq!(mods.final_price(Credits::new(10000)), Credits::new(10000));
    }

    #[test]
    fn supply_scales_independently() {
        let mods = PriceModifiers {
            supply: 0.6,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(600));
    }

    #[test]
    fn demand_scales_independently() {
        let mods = PriceModifiers {
            demand: 1.5,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(1500));
    }

    #[test]
    fn faction_scales_independently() {
        let mods = PriceModifiers {
            faction: 0.8,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(800));
    }

    #[test]
    fn event_scales_independently() {
        let mods = PriceModifiers {
            event: 2.5,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(2500));
    }

    #[test]
    fn reputation_scales_independently() {
        let mods = PriceModifiers {
            reputation: 1.1,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(1100));
    }

    #[test]
    fn modifiers_compound_multiplicatively() {
        // 1000 * 0.5 * 2.0 * 1.2 * 1.0 * 0.9 = 1080
        let mods = PriceModifiers {
            supply: 0.5,
            demand: 2.0,
            faction: 1.2,
            event: 1.0,
            reputation: 0.9,
        };
        assert_eq!(mods.final_price(Credits::new(1000)), Credits::new(1080));
    }

    #[test]
    fn rounding_is_to_nearest() {
        // 100 * 1.006 = 100.6 → rounds to 101
        let mods = PriceModifiers {
            reputation: 1.006,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(100)), Credits::new(101));
        // 100 * 1.004 = 100.4 → rounds to 100
        let mods = PriceModifiers {
            reputation: 1.004,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(100)), Credits::new(100));
    }

    #[test]
    fn price_never_zero() {
        let mods = PriceModifiers {
            supply: 0.001,
            ..PriceModifiers::default()
        };
        assert_eq!(mods.final_price(Credits::new(100)), Credits::new(1));
    }
}
