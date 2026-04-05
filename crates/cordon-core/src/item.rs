use serde::{Deserialize, Serialize};

use crate::faction::FactionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Food,
    Med,
    Ammo,
    Weapon,
    Helmet,
    Suit,
    Relic,
    Document,
    Tech,
    Grenade,
    Attachment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Caliber {
    /// 9x18mm PM
    Pm9x18,
    /// 9x19mm Parabellum
    Para9x19,
    /// .45 ACP
    Acp45,
    /// .50 AE (Hand Cannon only)
    Ae50,
    /// 5.45x39mm Soviet
    Soviet545,
    /// 5.56x45mm NATO
    Nato556,
    /// 7.62x39mm Soviet
    Soviet762x39,
    /// 7.62x51mm NATO
    Nato762x51,
    /// 7.62x54mmR Soviet
    SovietR762x54,
    /// 9x39mm subsonic
    Subsonic9x39,
    /// .308 Winchester
    Win308,
    /// 12 gauge
    Gauge12,
    /// VOG-25 caseless grenade
    Vog25,
    /// 40mm NATO grenade
    Grenade40mm,
    /// PG-7 rocket
    RocketPg7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorSlot {
    Suit,
    Helmet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicStability {
    Stable,
    Unstable,
    Inert,
    Counterfeit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Authenticity {
    Genuine,
    Counterfeit,
    Expired,
    Doctored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    VeryRare,
    Legendary,
}

/// Static item definition from the catalog. Immutable game data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub kind: ItemKind,
    pub base_price: u32,
    pub caliber: Option<Caliber>,
    pub fits_caliber: Option<Caliber>,
    pub suppliers: Vec<FactionId>,
    pub spoil_days: Option<u32>,
    pub rarity: Option<Rarity>,
    pub armor_slot: Option<ArmorSlot>,
}

/// A concrete item instance in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub def_id: ItemId,
    pub quantity: u32,
    pub condition: f32,
    pub authenticity: Authenticity,
    pub relic_stability: Option<RelicStability>,
    /// Days until spoilage. None = doesn't spoil.
    pub freshness: Option<u32>,
}

impl ItemStack {
    pub fn new(def_id: ItemId, quantity: u32, condition: f32) -> Self {
        Self {
            def_id,
            quantity,
            condition: condition.clamp(0.0, 1.0),
            authenticity: Authenticity::Genuine,
            relic_stability: None,
            freshness: None,
        }
    }
}
