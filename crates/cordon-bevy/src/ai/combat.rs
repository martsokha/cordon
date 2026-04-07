//! Combat: vision, engagement, weapon firing, damage application, tracers.
//!
//! Each NPC has a [`Vision`] radius. [`update_engagement`] scans for
//! hostile NPCs in vision; if a hostile is in weapon range with clear
//! line-of-sight, it pushes [`Action::Engage`]. Otherwise it pushes
//! [`Action::Walk`] toward the target. Damage is resolved each tick by
//! [`resolve_engage_actions`] using the loadout's equipped weapon and
//! the magazine's loaded ammo type.
//!
//! Anomaly disks block line-of-sight via [`AnomalyZone`].
//!
//! Death and looting are handled by sibling modules: [`super::death`]
//! tags corpses with `Dead`, and [`super::loot`] drives the loot action.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::entity::npc::Npc;
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Id, Rank, Resistances, Uid};
use cordon_data::gamedata::GameDataResource;
use moonshine_behavior::prelude::*;

use super::behavior::Action;
use super::death::Dead;
use crate::PlayingState;
use crate::laptop::{MapWorldEntity, NpcDot, NpcFaction};
use crate::world::SimWorld;

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 120 base + 15 per rank tier above Novice + 25 if
    /// the NPC's faction has military training. Scales to the map's
    /// area-radius scale (~60–165 units), so two NPCs in the same area
    /// can spot each other reliably.
    pub fn for_npc(rank: Rank, is_military: bool) -> Self {
        let from_rank = 120.0 + (rank.tier() as f32 - 1.0) * 15.0;
        let from_faction = if is_military { 25.0 } else { 0.0 };
        Self {
            radius: from_rank + from_faction,
        }
    }
}

/// Marker for anomaly entities, contributing to LOS blocking.
#[derive(Component, Debug, Clone, Copy)]
pub struct AnomalyZone {
    pub radius: f32,
}

/// A short-lived line drawn from shooter to target on each shot.
#[derive(Component, Debug, Clone, Copy)]
pub struct Tracer {
    /// Seconds remaining before despawn.
    pub life_secs: f32,
}

/// How long a tracer stays on screen, in seconds.
const TRACER_LIFE_SECS: f32 = 0.18;
/// Tracer width in map units.
const TRACER_WIDTH: f32 = 0.7;
/// Tracer fill colour (warm yellow-white).
const TRACER_COLOR: Color = Color::srgba(1.0, 0.92, 0.55, 0.95);

/// Whether two factions are hostile (relation ≤ -50 either way).
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

/// Effective firing range of the equipped weapon, in map units. 0 if unarmed.
fn weapon_range(items: &HashMap<Id<Item>, ItemDef>, loadout: &Loadout) -> f32 {
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

/// Plugin registering combat systems (vision, engagement, firing, tracers).
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_engagement, resolve_engage_actions, fade_tracers)
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Per-frame engagement decision for every alive NPC.
#[allow(clippy::type_complexity)]
fn update_engagement(
    sim: Option<Res<SimWorld>>,
    game_data: Res<GameDataResource>,
    anomalies: Query<(&Transform, &AnomalyZone)>,
    others: Query<(Entity, &NpcDot, &NpcFaction, &Transform), Without<Dead>>,
    mut q: Query<
        (
            Entity,
            &NpcDot,
            &NpcFaction,
            &Vision,
            &Transform,
            BehaviorMut<Action>,
        ),
        Without<Dead>,
    >,
) {
    let Some(sim) = sim else { return };
    let factions = &game_data.0.factions;
    let items = &game_data.0.items;
    let anomaly_disks: Vec<(Vec2, f32)> = anomalies
        .iter()
        .map(|(t, a)| (t.translation.truncate(), a.radius))
        .collect();

    // Snapshot positions of all alive NPCs so we can scan without
    // overlapping borrows on the main query.
    let snapshot: Vec<(Entity, Uid<Npc>, Id<Faction>, Vec2)> = others
        .iter()
        .map(|(e, dot, f, t)| (e, dot.uid, f.0.clone(), t.translation.truncate()))
        .collect();

    for (entity, npc_dot, faction, vision, transform, mut behavior) in &mut q {
        let pos = transform.translation.truncate();

        let Some(npc) = sim.0.npcs.get(&npc_dot.uid) else {
            continue;
        };
        if !npc.health.is_alive() {
            continue;
        }
        let range = weapon_range(items, &npc.loadout);
        if range <= 0.0 {
            // Unarmed: can't engage.
            continue;
        }

        // Find the nearest hostile in vision and with clear LOS.
        let mut best: Option<(Uid<Npc>, Vec2, f32)> = None;
        for (other_entity, other_uid, other_faction, other_pos) in &snapshot {
            if *other_entity == entity {
                continue;
            }
            if !is_hostile(&faction.0, other_faction, factions) {
                continue;
            }
            let dist = pos.distance(*other_pos);
            if dist > vision.radius {
                continue;
            }
            if line_blocked(pos, *other_pos, &anomaly_disks) {
                continue;
            }
            if best.is_none_or(|(_, _, d)| dist < d) {
                best = Some((*other_uid, *other_pos, dist));
            }
        }

        let Some((target_uid, target_pos, dist)) = best else {
            // No hostile in sight: drop out of Engage if we were in it.
            if matches!(behavior.current(), Action::Engage { .. }) {
                let _ = behavior.try_start(Action::Idle { timer: 0.5 });
            }
            continue;
        };

        if dist <= range {
            // In range: enter or stay in Engage. The cooldown starts at
            // 0 so the first shot fires this same tick.
            let already_engaging = matches!(
                behavior.current(),
                Action::Engage { target, .. } if *target == target_uid
            );
            if !already_engaging {
                let _ = behavior.try_start(Action::Engage {
                    target: target_uid,
                    cooldown_secs: 0.0,
                    reload_secs: 0.0,
                });
            }
        } else {
            // Out of range: walk toward the target.
            let already_walking = matches!(behavior.current(), Action::Walk { .. });
            if !already_walking {
                let _ = behavior.try_start(Action::Walk {
                    target: target_pos,
                    speed: 35.0,
                });
            }
        }
    }
}

/// Tick down engage cooldowns and apply damage when ready.
///
/// One pass over shooters. For each engaging NPC the state machine is:
///   1. If `reload_secs > 0`: tick the reload, skip.
///   2. Else if magazine is empty: pull a matching ammo box from the
///      pouch into the weapon and start a reload timer; skip.
///   3. Else if `cooldown_secs > 0`: tick the cooldown, skip.
///   4. Else: fire one shot — drain a round, wear the weapon, apply
///      HP damage and armor wear to the target, spawn a tracer, reset
///      the cooldown.
///
/// A broken or missing weapon drops the NPC out of `Engage`. An NPC
/// with no matching ammo and an empty mag also drops out (later this
/// will switch to the secondary).
#[allow(clippy::type_complexity)]
fn resolve_engage_actions(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut shooters: Query<(&NpcDot, &Transform, BehaviorMut<Action>), Without<Dead>>,
    targets: Query<(&NpcDot, &Transform), Without<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    // Snapshot positions of all alive NPCs for tracer endpoints.
    let positions: HashMap<Uid<Npc>, Vec2> = targets
        .iter()
        .map(|(dot, t)| (dot.uid, t.translation.truncate()))
        .collect();

    for (shooter_dot, shooter_transform, mut behavior) in &mut shooters {
        // Pull the engage state out of the action.
        let target_uid = match behavior.current() {
            Action::Engage { target, .. } => *target,
            _ => continue,
        };

        // Read the weapon stats once.
        let Some(shooter_npc) = sim.0.npcs.get(&shooter_dot.uid) else {
            continue;
        };
        let weapon_inst = match shooter_npc.loadout.equipped_weapon() {
            Some(w) => w,
            None => {
                // Broken or missing primary: drop combat.
                let _ = behavior.try_start(Action::Idle { timer: 0.5 });
                continue;
            }
        };
        let Some(weapon_def) = items.get(&weapon_inst.def_id) else {
            continue;
        };
        let (caliber, magazine, fire_rate, reload_secs_def, weapon_added) =
            match &weapon_def.data {
                ItemData::Weapon(w) => (
                    w.caliber.clone(),
                    w.magazine,
                    w.fire_rate,
                    w.reload_secs,
                    w.added_damage,
                ),
                _ => continue,
            };
        let mag_count = weapon_inst.count;
        let loaded_ammo_id = weapon_inst.loaded_ammo.clone();

        // Phase 1: tick reload timer if reloading.
        let in_reload = matches!(
            behavior.current(),
            Action::Engage { reload_secs, .. } if *reload_secs > 0.0
        );
        if in_reload {
            if let Action::Engage {
                reload_secs: rs, ..
            } = behavior.current_mut()
            {
                *rs = (*rs - dt).max(0.0);
            }
            continue;
        }

        // Phase 2: empty mag → start a reload.
        if mag_count == 0 {
            let started =
                refill_magazine(&mut sim, &shooter_dot, items, &caliber, magazine);
            if started {
                if let Action::Engage {
                    reload_secs: rs, ..
                } = behavior.current_mut()
                {
                    *rs = reload_secs_def;
                }
            } else {
                // No matching ammo anywhere → drop combat.
                let _ = behavior.try_start(Action::Idle { timer: 1.0 });
            }
            continue;
        }

        // Phase 3: cooldown still ticking?
        if let Action::Engage {
            cooldown_secs: cs, ..
        } = behavior.current_mut()
        {
            if *cs > 0.0 {
                *cs = (*cs - dt).max(0.0);
                continue;
            }
        }

        // Phase 4: fire a shot.
        // Resolve the loaded ammo's stats.
        let Some(loaded_ammo_id) = loaded_ammo_id else {
            // Mag has rounds but no recorded type: bail and let the
            // next reload tag it.
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

        // Look up the target and resolve the hit.
        let Some(target_npc) = sim.0.npcs.get(&target_uid) else {
            continue;
        };
        let target_ballistic = equipped_ballistic(&target_npc.loadout, items);
        let (dealt, absorbed) =
            Resistances::resolve_hit(target_ballistic, penetration, raw_damage);

        let shooter_pos = shooter_transform.translation.truncate();
        let Some(&target_pos) = positions.get(&target_uid) else {
            continue;
        };

        // Apply mutations to shooter.
        if let Some(shooter_mut) = sim.0.npcs.get_mut(&shooter_dot.uid)
            && let Some(weapon) = &mut shooter_mut.loadout.primary
        {
            weapon.count = weapon.count.saturating_sub(1);
            weapon.degrade(1);
        }
        // Apply mutations to target.
        if let Some(target_mut) = sim.0.npcs.get_mut(&target_uid) {
            target_mut.health.damage(dealt);
            if absorbed > 0
                && let Some(armor) = &mut target_mut.loadout.armor
            {
                armor.degrade(absorbed);
            }
        }

        // Spawn the tracer.
        spawn_tracer(
            &mut commands,
            &mut meshes,
            &mut materials,
            TRACER_COLOR,
            shooter_pos,
            target_pos,
        );

        // Reset the shot cooldown.
        if let Action::Engage {
            cooldown_secs: cs, ..
        } = behavior.current_mut()
        {
            *cs = if fire_rate > 0.0 { 1.0 / fire_rate } else { 1.0 };
        }
    }
}

/// Pull one matching ammo box from the shooter's general pouch and
/// refill the primary weapon up to its magazine size. Tags the weapon's
/// `loaded_ammo` with the box's def id so subsequent shots use accurate
/// damage and penetration values.
///
/// Returns `true` if any rounds were transferred.
fn refill_magazine(
    sim: &mut ResMut<SimWorld>,
    shooter_dot: &NpcDot,
    items: &HashMap<Id<Item>, ItemDef>,
    caliber: &Id<cordon_core::item::Caliber>,
    magazine: u32,
) -> bool {
    let Some(shooter) = sim.0.npcs.get_mut(&shooter_dot.uid) else {
        return false;
    };
    let Some(idx) = find_ammo_idx(&shooter.loadout, caliber, items) else {
        return false;
    };
    let box_def_id = shooter.loadout.general[idx].def_id.clone();
    let current_mag = shooter
        .loadout
        .primary
        .as_ref()
        .map(|w| w.count)
        .unwrap_or(0);
    let space = magazine.saturating_sub(current_mag);
    let take = space.min(shooter.loadout.general[idx].count);
    if take == 0 {
        return false;
    }
    shooter.loadout.general[idx].count -= take;
    if shooter.loadout.general[idx].count == 0 {
        shooter.loadout.general.remove(idx);
    }
    if let Some(weapon) = &mut shooter.loadout.primary {
        weapon.count += take;
        weapon.loaded_ammo = Some(box_def_id);
    }
    true
}

/// Spawn a tracer rectangle stretching from `from` to `to`.
fn spawn_tracer(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    color: Color,
    from: Vec2,
    to: Vec2,
) {
    let delta = to - from;
    let length = delta.length();
    if length < 0.5 {
        return;
    }
    let mid = (from + to) * 0.5;
    let angle = delta.y.atan2(delta.x);
    commands.spawn((
        MapWorldEntity,
        Tracer {
            life_secs: TRACER_LIFE_SECS,
        },
        Mesh2d(meshes.add(Rectangle::new(length, TRACER_WIDTH))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
        Transform {
            translation: Vec3::new(mid.x, mid.y, 0.6),
            rotation: Quat::from_rotation_z(angle),
            ..default()
        },
    ));
}

/// Fade tracers each tick and despawn expired ones.
fn fade_tracers(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut q: Query<(Entity, &mut Tracer, &MeshMaterial2d<ColorMaterial>)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tracer, mat_handle) in &mut q {
        tracer.life_secs -= dt;
        if tracer.life_secs <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (tracer.life_secs / TRACER_LIFE_SECS).clamp(0.0, 1.0);
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let c = mat.color.to_srgba();
            mat.color = Color::srgba(c.red, c.green, c.blue, alpha);
        }
    }
}
