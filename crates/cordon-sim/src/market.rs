use std::collections::HashMap;

use cordon_core::economy::item::ItemId;
use cordon_core::economy::price::{self, PriceModifiers};

pub struct MarketState {
    pub supply: HashMap<ItemId, f32>,
    pub demand: HashMap<ItemId, f32>,
    pub event_modifier: f32,
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            supply: HashMap::new(),
            demand: HashMap::new(),
            event_modifier: 1.0,
        }
    }

    pub fn get_modifiers(&self, item_id: ItemId, faction_modifier: f32, reputation: f32) -> PriceModifiers {
        PriceModifiers {
            supply: self.supply.get(&item_id).copied().unwrap_or(1.0),
            demand: self.demand.get(&item_id).copied().unwrap_or(1.0),
            faction: faction_modifier,
            event: self.event_modifier,
            reputation,
        }
    }

    pub fn get_price(
        &self,
        base_price: u32,
        condition: f32,
        item_id: ItemId,
        faction_modifier: f32,
        reputation: f32,
    ) -> u32 {
        let mods = self.get_modifiers(item_id, faction_modifier, reputation);
        price::final_price(base_price, condition, &mods)
    }
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new()
    }
}
