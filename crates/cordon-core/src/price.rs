/// Price modifiers from the world state.
#[derive(Debug, Clone, Copy)]
pub struct PriceModifiers {
    /// 0.5 – 1.5
    pub supply: f32,
    /// 0.5 – 2.0
    pub demand: f32,
    /// 0.8 – 1.2
    pub faction: f32,
    /// 0.5 – 3.0
    pub event: f32,
    /// 0.9 – 1.1
    pub reputation: f32,
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

/// Compute final price.
///
/// Condition is squared: a 0.5 item is worth ~25% of base, 0.75 is ~56%.
/// This makes low-condition gear nearly worthless and creates a repair arbitrage game.
pub fn final_price(base_price: u32, condition: f32, mods: &PriceModifiers) -> u32 {
    let condition = condition.clamp(0.0, 1.0);
    let condition_factor = condition * condition;

    let price = base_price as f64
        * condition_factor as f64
        * mods.supply as f64
        * mods.demand as f64
        * mods.faction as f64
        * mods.event as f64
        * mods.reputation as f64;

    (price.round() as u32).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_squared_pricing() {
        let mods = PriceModifiers::default();
        let base = 10000;

        let full = final_price(base, 1.0, &mods);
        let three_quarter = final_price(base, 0.75, &mods);
        let half = final_price(base, 0.5, &mods);
        let quarter = final_price(base, 0.25, &mods);

        assert_eq!(full, 10000);
        assert_eq!(three_quarter, 5625);
        assert_eq!(half, 2500);
        assert_eq!(quarter, 625);
    }

    #[test]
    fn price_never_zero() {
        let mods = PriceModifiers::default();
        assert_eq!(final_price(100, 0.01, &mods), 1);
    }
}
