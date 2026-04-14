//! Pure combat helpers: hostility, line-of-sight, weapon range,
//! equipped resistances. Shared by the combat resolver and by squad
//! engagement.

use std::collections::HashMap;

use bevy::math::Vec2;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Id, Resistances};

/// Whether two factions are hostile.
pub fn is_hostile(
    a: &Id<Faction>,
    b: &Id<Faction>,
    factions: &HashMap<Id<Faction>, FactionDef>,
) -> bool {
    if a == b {
        return false;
    }
    let lookup = |source: &FactionDef, target: &Id<Faction>| -> bool {
        source
            .relations
            .iter()
            .find(|(other, _)| other == target)
            .map(|(_, rel)| rel.is_hostile())
            .unwrap_or(false)
    };
    if let Some(def_a) = factions.get(a)
        && lookup(def_a, b)
    {
        return true;
    }
    if let Some(def_b) = factions.get(b)
        && lookup(def_b, a)
    {
        return true;
    }
    false
}

/// True if the segment from `from` to `to` passes through any anomaly
/// disk *that neither endpoint is standing inside*. An anomaly only
/// blocks line-of-sight when the viewer is outside it looking through
/// — squads patrolling inside the same fog can still see each other,
/// otherwise everyone in an anomaly is permanently blind.
pub fn line_blocked(from: Vec2, to: Vec2, anomalies: &[(Vec2, f32)]) -> bool {
    let dir = to - from;
    let len_sq = dir.length_squared();
    if len_sq < f32::EPSILON {
        return false;
    }
    for (center, radius) in anomalies {
        let r_sq = radius * radius;
        // Skip if either endpoint is inside this anomaly: the
        // observer (or target) is already in the fog and isn't
        // line-blocked by their own surroundings.
        if from.distance_squared(*center) <= r_sq || to.distance_squared(*center) <= r_sq {
            continue;
        }
        let to_center = *center - from;
        let t = (to_center.dot(dir) / len_sq).clamp(0.0, 1.0);
        let closest = from + dir * t;
        if closest.distance_squared(*center) <= r_sq {
            return true;
        }
    }
    false
}

/// Effective firing range of the equipped weapon, in map units.
pub fn weapon_range(items: &HashMap<Id<Item>, ItemDef>, loadout: &Loadout) -> f32 {
    let Some(inst) = loadout.equipped_weapon() else {
        return 0.0;
    };
    let Some(def) = items.get(&inst.def_id) else {
        return 0.0;
    };
    match &def.data {
        ItemData::Weapon(w) => w.range.value(),
        _ => 0.0,
    }
}

/// Combined ballistic resistance from equipped suit, helmet, and
/// relic passives. The relic closure resolves each `ItemInstance` in
/// the loadout's relic slots to its `RelicData` via the item
/// catalog; unknown ids are skipped.
pub(super) fn equipped_ballistic(loadout: &Loadout, items: &HashMap<Id<Item>, ItemDef>) -> u32 {
    let armor = loadout
        .armor
        .as_ref()
        .and_then(|i| items.get(&i.def_id))
        .and_then(|def| match &def.data {
            ItemData::Armor(a) => Some(a),
            _ => None,
        });
    let helmet = loadout
        .helmet
        .as_ref()
        .and_then(|i| items.get(&i.def_id))
        .and_then(|def| match &def.data {
            ItemData::Armor(a) => Some(a),
            _ => None,
        });
    let resistances: Resistances = loadout.equipped_resistances(armor, helmet, |inst| {
        items.get(&inst.def_id).and_then(|def| match &def.data {
            ItemData::Relic(r) => Some(r),
            _ => None,
        })
    });
    resistances.ballistic
}

/// Find the index in the general pouch of an ammo box for the given caliber.
pub(super) fn find_ammo_idx(
    loadout: &Loadout,
    caliber: &Id<cordon_core::item::Caliber>,
    items: &HashMap<Id<Item>, ItemDef>,
) -> Option<usize> {
    loadout.general.iter().position(|inst| {
        let Some(def) = items.get(&inst.def_id) else {
            return false;
        };
        match &def.data {
            ItemData::Ammo(a) => a.caliber == *caliber && inst.count > 0,
            _ => false,
        }
    })
}
