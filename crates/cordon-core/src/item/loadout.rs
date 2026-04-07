//! NPC equipment with typed slots and a general carry pouch.

use serde::{Deserialize, Serialize};

use super::data::{ArmorData, ArmorSlot, RelicData};
use super::effect::{PassiveModifier, StatTarget};
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

    /// Combined ballistic/hazard resistances from equipped armor,
    /// helmet, and relics.
    ///
    /// `resolve_relic` is called for each relic in
    /// [`self.relics`](Self::relics); returning `None` (e.g. the relic
    /// def isn't found in the catalog) skips that relic. Relic passive
    /// modifiers targeting `*Resistance` stats are folded in; other
    /// passive targets (e.g. `MaxHealth`) are ignored here.
    ///
    /// Relic contributions can be negative: the slug relic has
    /// `BallisticResistance: -5` as a deliberate drawback. The
    /// intermediate sum is signed so a drawback actually subtracts
    /// before the final clamp to `u32`.
    pub fn equipped_resistances<'a, F>(
        &'a self,
        armor_def: Option<&ArmorData>,
        helmet_def: Option<&ArmorData>,
        resolve_relic: F,
    ) -> Resistances
    where
        F: FnMut(&'a ItemInstance) -> Option<&'a RelicData>,
    {
        let mut base = Resistances::NONE;
        if let (Some(_), Some(def)) = (&self.armor, armor_def) {
            base = base.combine(def.resistances);
        }
        if let (Some(_), Some(def)) = (&self.helmet, helmet_def) {
            base = base.combine(def.resistances);
        }

        let relic_passives = self
            .relics
            .iter()
            .filter_map(resolve_relic)
            .map(|r| r.passive.as_slice());
        base.apply_passive_modifiers(relic_passives)
    }

    /// The currently equipped primary weapon, if any.
    pub fn equipped_weapon(&self) -> Option<&ItemInstance> {
        self.primary.as_ref()
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

/// Extend [`Resistances`] with a fold over [`PassiveModifier`] sources.
///
/// Lives in the item module so `cordon_core::primitive` doesn't need
/// to depend on item-level types. The method is still written as
/// `impl Resistances` so call sites read as
/// `base.apply_passive_modifiers(...)`.
impl Resistances {
    /// Fold an iterator of passive modifier slices into `self`,
    /// returning the updated resistances.
    ///
    /// Intermediate accumulation is signed (`i64`) so negative
    /// modifiers (a relic with a drawback like `-5 Ballistic`)
    /// cancel positive contributions before the final clamp back
    /// to `u32`. Non-resistance targets (e.g. `MaxHealth`) are
    /// silently skipped — they're handled by separate systems.
    ///
    /// Takes an iterator so the hot path in `equipped_resistances`
    /// can compose it directly from `self.relics.iter()` without
    /// any intermediate Vec allocation.
    pub fn apply_passive_modifiers<'a, I>(self, passives: I) -> Self
    where
        I: IntoIterator<Item = &'a [PassiveModifier]>,
    {
        let mut acc: [i64; 6] = [
            self.ballistic as i64,
            self.radiation as i64,
            self.chemical as i64,
            self.thermal as i64,
            self.electric as i64,
            self.gravitational as i64,
        ];

        for slice in passives {
            for modifier in slice {
                let value = modifier.value.round() as i64;
                match modifier.target {
                    StatTarget::BallisticResistance => acc[0] += value,
                    StatTarget::RadiationResistance => acc[1] += value,
                    StatTarget::ChemicalResistance => acc[2] += value,
                    StatTarget::ThermalResistance => acc[3] += value,
                    StatTarget::ElectricResistance => acc[4] += value,
                    StatTarget::GravitationalResistance => acc[5] += value,
                    StatTarget::MaxHealth | StatTarget::MaxStamina | StatTarget::MaxHunger => {}
                }
            }
        }

        let clamp = |n: i64| n.max(0) as u32;
        Resistances {
            ballistic: clamp(acc[0]),
            radiation: clamp(acc[1]),
            chemical: clamp(acc[2]),
            thermal: clamp(acc[3]),
            electric: clamp(acc[4]),
            gravitational: clamp(acc[5]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifier(target: StatTarget, value: f32) -> PassiveModifier {
        PassiveModifier { target, value }
    }

    fn fold_one(base: Resistances, relic: &[PassiveModifier]) -> Resistances {
        base.apply_passive_modifiers(std::iter::once(relic))
    }

    fn fold_many<'a>(
        base: Resistances,
        relics: impl IntoIterator<Item = &'a [PassiveModifier]>,
    ) -> Resistances {
        base.apply_passive_modifiers(relics)
    }

    #[test]
    fn positive_relic_passives_accumulate() {
        let base = Resistances {
            ballistic: 10,
            ..Resistances::NONE
        };
        let relic: &[PassiveModifier] = &[modifier(StatTarget::BallisticResistance, 20.0)];
        let result = fold_one(base, relic);
        assert_eq!(result.ballistic, 30);
    }

    #[test]
    fn negative_relic_passives_subtract() {
        // The slug relic has -5 ballistic as a drawback.
        let base = Resistances {
            ballistic: 20,
            ..Resistances::NONE
        };
        let relic: &[PassiveModifier] = &[
            modifier(StatTarget::GravitationalResistance, 30.0),
            modifier(StatTarget::BallisticResistance, -5.0),
        ];
        let result = fold_one(base, relic);
        assert_eq!(result.ballistic, 15, "drawback should subtract");
        assert_eq!(result.gravitational, 30);
    }

    #[test]
    fn negative_relic_passives_clamp_at_zero() {
        // Overkill drawback should floor at 0, not wrap.
        let base = Resistances {
            ballistic: 3,
            ..Resistances::NONE
        };
        let relic: &[PassiveModifier] = &[modifier(StatTarget::BallisticResistance, -50.0)];
        let result = fold_one(base, relic);
        assert_eq!(result.ballistic, 0);
    }

    #[test]
    fn multiple_relics_cancel_correctly() {
        // Two relics: +20 and -10 should net +10.
        let base = Resistances::NONE;
        let relic_a: &[PassiveModifier] = &[modifier(StatTarget::ThermalResistance, 20.0)];
        let relic_b: &[PassiveModifier] = &[modifier(StatTarget::ThermalResistance, -10.0)];
        let result = fold_many(base, [relic_a, relic_b]);
        assert_eq!(result.thermal, 10);
    }

    #[test]
    fn max_health_passive_ignored_by_resistance_fold() {
        let base = Resistances::NONE;
        let relic: &[PassiveModifier] = &[
            modifier(StatTarget::MaxHealth, 10.0),
            modifier(StatTarget::BallisticResistance, 5.0),
        ];
        let result = fold_one(base, relic);
        assert_eq!(result.ballistic, 5);
    }
}
