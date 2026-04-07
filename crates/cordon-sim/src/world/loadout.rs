//! Generate NPC loadouts from faction archetypes.

use std::collections::HashMap;

use cordon_core::entity::archetype::{ArchetypeDef, WeightedItem};
use cordon_core::item::{
    ArmorData, ArmorSlot, Caliber, Item, ItemData, ItemDef, ItemInstance, Loadout,
};
use cordon_core::primitive::{Id, Rank};
use rand::{Rng, RngExt};

/// Roll a complete [`Loadout`] for an NPC.
///
/// Returns an empty loadout if the archetype has no entry for this rank
/// (or any lower fallback rank). Items the archetype references but the
/// catalog doesn't define are skipped silently — a generator failure
/// shouldn't crash the sim, it should just produce a less-equipped NPC.
pub fn generate_loadout<R: Rng>(
    archetype: &ArchetypeDef,
    rank: Rank,
    items: &HashMap<Id<Item>, ItemDef>,
    rng: &mut R,
) -> Loadout {
    let Some(rank_loadout) = archetype.for_rank(rank) else {
        return Loadout::new();
    };

    let mut loadout = Loadout::new();

    // Primary weapon — drives ammo selection.
    loadout.primary = roll_item(&rank_loadout.primary, items, rng);
    // Secondary weapon (sidearm).
    loadout.secondary = roll_item(&rank_loadout.secondary, items, rng);

    // Armor (suit) and helmet, in that order so resistances/capacity
    // can be inspected by callers later.
    loadout.armor = roll_armor(&rank_loadout.armor, items, ArmorSlot::Suit, rng);
    loadout.helmet = roll_armor(&rank_loadout.helmet, items, ArmorSlot::Helmet, rng);

    // Compute general capacity from rank + equipped armor's bonus.
    let armor_data = loadout
        .armor
        .as_ref()
        .and_then(|inst| items.get(&inst.def_id))
        .and_then(|def| match &def.data {
            ItemData::Armor(a) => Some(a),
            _ => None,
        });
    let capacity = Loadout::general_capacity(rank, armor_data);

    // Load the primary weapon's mag and stock spare ammo boxes.
    fill_weapon_and_reserves(
        &mut loadout.primary,
        &mut loadout.general,
        items,
        rank_loadout.ammo_boxes,
        capacity,
        rng,
    );
    // Same for the secondary.
    fill_weapon_and_reserves(
        &mut loadout.secondary,
        &mut loadout.general,
        items,
        rank_loadout.secondary_ammo_boxes,
        capacity,
        rng,
    );

    // Consumables: roll `consumable_count` items from the pool.
    for _ in 0..rank_loadout.consumable_count {
        if let Some(item) = roll_item(&rank_loadout.consumables, items, rng) {
            let _ = loadout.add_to_general(item, capacity);
        }
    }

    loadout
}

/// Roll an ammo type for a freshly-rolled weapon, then:
///   1. Set the weapon's `loaded_ammo` to that type and `count` to a full mag.
///   2. Append `reserve_boxes` extra fresh ammo boxes (any caliber-matching
///      def, picked uniformly each time) to the general pouch.
fn fill_weapon_and_reserves<R: Rng>(
    weapon_slot: &mut Option<ItemInstance>,
    general: &mut Vec<ItemInstance>,
    items: &HashMap<Id<Item>, ItemDef>,
    reserve_boxes: u32,
    capacity: u8,
    rng: &mut R,
) {
    let Some(weapon_inst) = weapon_slot.as_mut() else {
        return;
    };
    let Some(weapon_def) = items.get(&weapon_inst.def_id) else {
        return;
    };
    let (caliber, magazine) = match &weapon_def.data {
        ItemData::Weapon(w) => (w.caliber.clone(), w.magazine),
        _ => return,
    };

    // Pick the ammo type to load and load a full mag.
    if let Some(loaded_def) = pick_ammo_def_for_caliber(&caliber, items, rng) {
        weapon_inst.loaded_ammo = Some(loaded_def.id.clone());
        weapon_inst.count = magazine;
    }

    // Spare boxes: same caliber, any type.
    for _ in 0..reserve_boxes {
        if let Some(def) = pick_ammo_def_for_caliber(&caliber, items, rng) {
            let inst = ItemInstance::new(def);
            if (general.len() as u8) >= capacity {
                break;
            }
            general.push(inst);
        }
    }
}

/// Pick a single ammo `ItemDef` matching the given caliber, uniformly.
fn pick_ammo_def_for_caliber<'a, R: Rng>(
    caliber: &Id<Caliber>,
    items: &'a HashMap<Id<Item>, ItemDef>,
    rng: &mut R,
) -> Option<&'a ItemDef> {
    let candidates: Vec<&ItemDef> = items
        .values()
        .filter(|def| match &def.data {
            ItemData::Ammo(a) => &a.caliber == caliber,
            _ => false,
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.random_range(0..candidates.len())])
}

/// Pick one item from a weighted pool, instantiate it via the catalog.
fn roll_item<R: Rng>(
    pool: &[WeightedItem],
    items: &HashMap<Id<Item>, ItemDef>,
    rng: &mut R,
) -> Option<ItemInstance> {
    let id = pick_weighted(pool, rng)?;
    let def = items.get(id)?;
    Some(ItemInstance::new(def))
}

/// Pick an armor item, sanity-checking the slot matches.
fn roll_armor<R: Rng>(
    pool: &[WeightedItem],
    items: &HashMap<Id<Item>, ItemDef>,
    expected_slot: ArmorSlot,
    rng: &mut R,
) -> Option<ItemInstance> {
    let id = pick_weighted(pool, rng)?;
    let def = items.get(id)?;
    let ArmorData { slot, .. } = match &def.data {
        ItemData::Armor(a) => a,
        _ => return None,
    };
    if *slot != expected_slot {
        return None;
    }
    Some(ItemInstance::new(def))
}

/// Roll a weighted choice from a pool, returning the chosen item ID.
fn pick_weighted<'a, R: Rng>(pool: &'a [WeightedItem], rng: &mut R) -> Option<&'a Id<Item>> {
    if pool.is_empty() {
        return None;
    }
    let total: u32 = pool.iter().map(|w| w.weight.max(1)).sum();
    if total == 0 {
        return Some(&pool[0].id);
    }
    let mut roll = rng.random_range(0..total);
    for entry in pool {
        let w = entry.weight.max(1);
        if roll < w {
            return Some(&entry.id);
        }
        roll -= w;
    }
    Some(&pool[pool.len() - 1].id)
}

