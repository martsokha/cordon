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
    /// Specialized equipment, mid-tier relics. Not always in stock.
    Rare,
    /// Elite weapons, deep-Zone relics. Factions take notice.
    VeryRare,
    /// Endgame relics, one-of-a-kind finds. Wars start over these.
    Legendary,
}
