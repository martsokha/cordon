//! Combat resolution: weapon firing, damage application, hostility checks.
//!
//! Engagement *decisions* (which target, when to advance) live in
//! [`crate::squad_ai`]. This module owns the per-NPC firing loop:
//! reading the [`CombatTarget`] component the squad system wrote,
//! ticking [`FireState`] cooldowns, applying damage when ready, and
//! emitting [`ShotFired`] events for the visual layer to render.

use std::collections::HashMap;

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Id, Resistances};
use cordon_data::gamedata::GameDataResource;

use crate::behavior::{CombatTarget, Dead, FireState};
use crate::components::{Hp, LoadoutComp};
use crate::events::ShotFired;
use crate::plugin::SimSet;

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

/// True if the segment from `from` to `to` passes through any anomaly disk.
pub fn line_blocked(from: Vec2, to: Vec2, anomalies: &[(Vec2, f32)]) -> bool {
    let dir = to - from;
    let len_sq = dir.length_squared();
    if len_sq < f32::EPSILON {
        return false;
    }
    for (center, radius) in anomalies {
        let to_center = *center - from;
        let t = (to_center.dot(dir) / len_sq).clamp(0.0, 1.0);
        let closest = from + dir * t;
        if closest.distance_squared(*center) <= radius * radius {
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

/// Combined ballistic resistance from equipped (non-broken) suit + helmet.
fn equipped_ballistic(loadout: &Loadout, items: &HashMap<Id<Item>, ItemDef>) -> u32 {
    let armor = loadout
        .armor
        .as_ref()
        .filter(|i| !i.is_broken())
        .and_then(|i| items.get(&i.def_id))
        .and_then(|def| match &def.data {
            ItemData::Armor(a) => Some(a),
            _ => None,
        });
    let helmet = loadout
        .helmet
        .as_ref()
        .filter(|i| !i.is_broken())
        .and_then(|i| items.get(&i.def_id))
        .and_then(|def| match &def.data {
            ItemData::Armor(a) => Some(a),
            _ => None,
        });
    let resistances: Resistances = loadout.equipped_resistances(armor, helmet);
    resistances.ballistic
}

/// Find the index in the general pouch of an ammo box for the given caliber.
fn find_ammo_idx(
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

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ShotFired>();
        app.add_systems(Update, resolve_combat.in_set(SimSet::Combat));
    }
}

/// One pending hit produced by the shooter loop and applied to the
/// target in a separate pass. Decoupling lets us hold one mutable
/// query at a time, avoiding overlapping `&mut LoadoutComp` access.
struct HitIntent {
    target: Entity,
    dealt: u32,
    absorbed: u32,
}

/// Tick down per-NPC fire cooldowns and apply damage when ready.
///
/// All NPC mutable access flows through a [`ParamSet`] so multiple
/// `&mut LoadoutComp` queries don't overlap. The shooter loop reads
/// from a position+armor snapshot built before mutation, drains the
/// shooter's own ammo and wears its weapon, then queues a [`HitIntent`].
/// A second pass applies the hit to the target via a fresh mutable query.
#[allow(clippy::type_complexity)]
fn resolve_combat(
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut shots: MessageWriter<ShotFired>,
    mut sets: ParamSet<(
        // Read-only snapshot pass.
        Query<(Entity, &Transform, &LoadoutComp), (With<Hp>, Without<Dead>)>,
        // Shooter mutation pass.
        Query<
            (
                Entity,
                &Transform,
                &mut CombatTarget,
                &mut FireState,
                &mut LoadoutComp,
            ),
            Without<Dead>,
        >,
        // Target apply pass.
        Query<(&mut Hp, &mut LoadoutComp), Without<Dead>>,
    )>,
) {
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    // Pass 0: snapshot every alive NPC's position + ballistic resistance.
    let target_snapshot: HashMap<Entity, (Vec2, u32)> = {
        let mut m = HashMap::with_capacity(1024);
        for (entity, transform, loadout) in sets.p0().iter() {
            let pos = transform.translation.truncate();
            let ballistic = equipped_ballistic(&loadout.0, items);
            m.insert(entity, (pos, ballistic));
        }
        m
    };

    let mut hits: Vec<HitIntent> = Vec::new();

    // Pass 1: shooter loop.
    let mut shooters = sets.p1();
    for (shooter_entity, shooter_transform, mut combat_target, mut fire_state, mut loadout) in
        &mut shooters
    {
        let Some(target_entity) = combat_target.0 else {
            continue;
        };

        let Some(&(target_pos, target_ballistic)) = target_snapshot.get(&target_entity) else {
            combat_target.0 = None;
            *fire_state = FireState::default();
            continue;
        };

        let Some(weapon_inst) = loadout.0.equipped_weapon() else {
            combat_target.0 = None;
            *fire_state = FireState::default();
            continue;
        };
        let Some(weapon_def) = items.get(&weapon_inst.def_id) else {
            continue;
        };
        let (caliber, magazine, fire_rate, reload_secs_def, weapon_added, range) =
            match &weapon_def.data {
                ItemData::Weapon(w) => (
                    w.caliber.clone(),
                    w.magazine,
                    w.fire_rate,
                    w.reload_secs,
                    w.added_damage,
                    w.range.value(),
                ),
                _ => continue,
            };
        let mag_count = weapon_inst.count;
        let loaded_ammo_id = weapon_inst.loaded_ammo.clone();

        let shooter_pos = shooter_transform.translation.truncate();
        if shooter_pos.distance(target_pos) > range {
            continue;
        }

        if fire_state.reload_secs > 0.0 {
            fire_state.reload_secs = (fire_state.reload_secs - dt).max(0.0);
            continue;
        }

        if mag_count == 0 {
            let started = refill_magazine(&mut loadout.0, items, &caliber, magazine);
            if started {
                fire_state.reload_secs = reload_secs_def;
            } else {
                combat_target.0 = None;
                *fire_state = FireState::default();
            }
            continue;
        }

        if fire_state.cooldown_secs > 0.0 {
            fire_state.cooldown_secs = (fire_state.cooldown_secs - dt).max(0.0);
            continue;
        }

        // Fire one shot.
        let Some(loaded_ammo_id) = loaded_ammo_id else {
            continue;
        };
        let Some(ammo_def) = items.get(&loaded_ammo_id) else {
            continue;
        };
        let (ammo_damage, penetration) = match &ammo_def.data {
            ItemData::Ammo(a) => (a.damage, a.penetration),
            _ => continue,
        };
        let raw_damage = ammo_damage + weapon_added;

        let (dealt, absorbed) = Resistances::resolve_hit(target_ballistic, penetration, raw_damage);

        if let Some(weapon) = &mut loadout.0.primary {
            weapon.count = weapon.count.saturating_sub(1);
            weapon.degrade(1);
        }

        shots.write(ShotFired {
            shooter: shooter_entity,
            from: shooter_pos,
            to: target_pos,
        });

        fire_state.cooldown_secs = if fire_rate > 0.0 {
            1.0 / fire_rate
        } else {
            1.0
        };
        hits.push(HitIntent {
            target: target_entity,
            dealt,
            absorbed,
        });
    }

    // Pass 2: apply HP damage and armor wear.
    drop(shooters);
    let mut targets_apply = sets.p2();
    for hit in hits {
        if let Ok((mut hp, mut loadout)) = targets_apply.get_mut(hit.target) {
            hp.current.damage(hit.dealt);
            if hit.absorbed > 0
                && let Some(armor) = &mut loadout.0.armor
            {
                armor.degrade(hit.absorbed);
            }
        }
    }
}

/// Pull one matching ammo box from the loadout's general pouch and
/// refill the primary weapon up to its magazine size.
fn refill_magazine(
    loadout: &mut Loadout,
    items: &HashMap<Id<Item>, ItemDef>,
    caliber: &Id<cordon_core::item::Caliber>,
    magazine: u32,
) -> bool {
    let Some(idx) = find_ammo_idx(loadout, caliber, items) else {
        return false;
    };
    let box_def_id = loadout.general[idx].def_id.clone();
    let current_mag = loadout.primary.as_ref().map(|w| w.count).unwrap_or(0);
    let space = magazine.saturating_sub(current_mag);
    let take = space.min(loadout.general[idx].count);
    if take == 0 {
        return false;
    }
    loadout.general[idx].count -= take;
    if loadout.general[idx].count == 0 {
        loadout.general.remove(idx);
    }
    if let Some(weapon) = &mut loadout.primary {
        weapon.count += take;
        weapon.loaded_ammo = Some(box_def_id);
    }
    true
}
