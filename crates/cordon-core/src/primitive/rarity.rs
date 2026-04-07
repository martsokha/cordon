//! Item rarity tiers.

use serde::{Deserialize, Serialize};

/// How rare an item is. Affects loot table weighting, base price
/// multipliers, and NPC reactions when they see it on your shelves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    /// Bread, bandages, pistol rounds. Always available somewhere.
    Common,
    /// Standard military gear, basic relics. Regular trade goods.
    Uncommon,
    /// Specialized equipment, high-tier relics. Not always in stock.
    Rare,
}

impl Rarity {
    /// Relative weight when rolling on a rarity-weighted table.
    ///
    /// Roughly "half as likely per step": common is 5× uncommon and
    /// ~2.5× rare. Used by the relic spawner and any other
    /// rarity-weighted draw.
    pub fn weight(self) -> u32 {
        match self {
            Rarity::Common => 10,
            Rarity::Uncommon => 5,
            Rarity::Rare => 2,
        }
    }
}
