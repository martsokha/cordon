//! NPC equipment with typed slots and a general carry pouch.

use serde::{Deserialize, Serialize};

use super::data::{ArmorData, ArmorSlot};
use super::instance::ItemInstance;
use crate::primitive::{Rank, Resistances};

/// Base general slots before any rank or armor bonus.
pub const BASE_GENERAL_SLOTS: u8 = 10;

/// Hard cap on relic slots regardless of armor.
pub const MAX_RELIC_SLOTS: u8 = 4;

/// What an NPC is wearing and carrying.
///
/// Equipment lives in typed slots (primary/secondary weapon, armor,
/// helmet, relics) so combat can find what it needs without scanning
/// a flat list. The general slots hold ammo, consumables, scavenged
/// loot, and unequipped equipment, with capacity scaling by rank and
/// equipped armor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Loadout {
    /// Primary weapon (if equipped). The weapon instance's `count`
    /// holds the rounds currently in its magazine, drained as the NPC
    /// fires and refilled from a general-pouch ammo box when reloading.
    pub primary: Option<ItemInstance>,
    /// Secondary weapon (sidearm or backup). `count` is loaded rounds.
    pub secondary: Option<ItemInstance>,
    /// Body armor (suit).
    pub armor: Option<ItemInstance>,
    /// Head protection.
    pub helmet: Option<ItemInstance>,
    /// Equipped relics. Capped at the suit's `relic_slots` (and at 4).
    pub relics: Vec<ItemInstance>,
    /// General carry: ammo, consumables, loot, spare gear.
    pub general: Vec<ItemInstance>,
}

impl Loadout {
    /// Create an empty loadout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the general carry capacity for an NPC of the given rank.
    ///
    /// `armor_def` is the equipped suit's def (if any), used to read
    /// its `inventory_slots` bonus.
    pub fn general_capacity(rank: Rank, armor_def: Option<&ArmorData>) -> u8 {
        let from_rank = rank.tier() - 1; // novice +0, legend +4
        let from_armor = armor_def.map(|a| a.inventory_slots).unwrap_or(0);
        BASE_GENERAL_SLOTS + from_rank + from_armor
    }

    /// Compute the relic slot capacity from the equipped suit.
    /// Helmets do not contribute. Capped at [`MAX_RELIC_SLOTS`].
    pub fn relic_capacity(armor_def: Option<&ArmorData>) -> u8 {
        armor_def
            .filter(|a| a.slot == ArmorSlot::Suit)
            .map(|a| a.relic_slots.min(MAX_RELIC_SLOTS))
            .unwrap_or(0)
    }

    /// Combined ballistic/hazard resistances from equipped armor + helmet.
    /// Broken pieces (durability == 0) provide no protection.
    pub fn equipped_resistances(
        &self,
        armor_def: Option<&ArmorData>,
        helmet_def: Option<&ArmorData>,
    ) -> Resistances {
        let mut total = Resistances::NONE;
        if let (Some(inst), Some(def)) = (&self.armor, armor_def)
            && !inst.is_broken()
        {
            total = total.combine(def.resistances);
        }
        if let (Some(inst), Some(def)) = (&self.helmet, helmet_def)
            && !inst.is_broken()
        {
            total = total.combine(def.resistances);
        }
        total
    }

    /// The currently equipped primary weapon, if any (and not broken).
    pub fn equipped_weapon(&self) -> Option<&ItemInstance> {
        self.primary.as_ref().filter(|w| !w.is_broken())
    }

    /// Try to add an item to the general pouch. Returns `Err(item)` if
    /// the pouch is full at the given capacity.
    pub fn add_to_general(&mut self, item: ItemInstance, capacity: u8) -> Result<(), ItemInstance> {
        if self.general.len() as u8 >= capacity {
            Err(item)
        } else {
            self.general.push(item);
            Ok(())
        }
    }

    /// Try to add a relic. Returns `Err(item)` if the relic slots are full.
    pub fn add_relic(&mut self, item: ItemInstance, capacity: u8) -> Result<(), ItemInstance> {
        if self.relics.len() as u8 >= capacity {
            Err(item)
        } else {
            self.relics.push(item);
            Ok(())
        }
    }

    /// Iterate every item in this loadout (equipment + general + relics)
    /// in a stable order: primary, secondary, armor, helmet, relics, general.
    pub fn iter(&self) -> impl Iterator<Item = &ItemInstance> {
        self.primary
            .iter()
            .chain(self.secondary.iter())
            .chain(self.armor.iter())
            .chain(self.helmet.iter())
            .chain(self.relics.iter())
            .chain(self.general.iter())
    }

    /// Whether the loadout has nothing in it.
    pub fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.secondary.is_none()
            && self.armor.is_none()
            && self.helmet.is_none()
            && self.relics.is_empty()
            && self.general.is_empty()
    }
}
