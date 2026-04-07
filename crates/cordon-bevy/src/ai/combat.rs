//! Combat: vision, weapon firing, damage application, tracers.
//!
//! Engagement *decisions* (which target, when to advance) live in
//! [`super::squad`]. This module owns the per-NPC firing loop:
//! reading the [`CombatTarget`] component the squad system wrote,
//! ticking the [`FireState`] cooldowns, applying damage when ready,
//! and spawning tracers.

use std::collections::HashMap;

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Id, Rank, Resistances};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::components::{Hp, LoadoutComp};

use super::AiSet;
use super::behavior::{CombatTarget, FireState};
use super::death::Dead;
use crate::PlayingState;
use crate::laptop::MapWorldEntity;

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 120 base + 15 per rank tier above Novice + 25 if
    /// the NPC's faction has military training.
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
    pub life_secs: f32,
}

const TRACER_LIFE_SECS: f32 = 0.18;
const TRACER_WIDTH: f32 = 0.7;
const TRACER_COLOR: Color = Color::srgba(1.0, 0.92, 0.55, 0.95);

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

/// Shared mesh + material for tracer entities.
#[derive(Resource, Clone)]
pub struct TracerAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

fn init_tracer_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, TRACER_WIDTH));
    let material = materials.add(ColorMaterial::from_color(TRACER_COLOR));
    commands.insert_resource(TracerAssets { mesh, material });
}

/// Plugin registering combat systems.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_tracer_assets);
        app.add_systems(
            Update,
            (
                resolve_combat.in_set(AiSet::Combat),
                fade_tracers.in_set(AiSet::Cleanup),
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
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
    tracer_assets: Res<TracerAssets>,
    mut commands: Commands,
    mut sets: ParamSet<(
        // Read-only snapshot pass.
        Query<(Entity, &Transform, &LoadoutComp), (With<Hp>, Without<Dead>)>,
        // Shooter mutation pass.
        Query<
            (
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

    // Pass 1: shooter loop. Reads from target_snapshot for target data
    // so the only mutable access is the shooter's own components.
    let mut shooters = sets.p1();
    for (shooter_transform, mut combat_target, mut fire_state, mut loadout) in &mut shooters {
        // Step 0: do we have a target?
        let Some(target_entity) = combat_target.0 else {
            continue;
        };

        // Step 1: target alive?
        let Some(&(target_pos, target_ballistic)) = target_snapshot.get(&target_entity) else {
            combat_target.0 = None;
            *fire_state = FireState::default();
            continue;
        };

        // Step 2: read weapon stats from our loadout.
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

        // Step 3: out of range?
        let shooter_pos = shooter_transform.translation.truncate();
        if shooter_pos.distance(target_pos) > range {
            continue;
        }

        // Step 4: ticking reload?
        if fire_state.reload_secs > 0.0 {
            fire_state.reload_secs = (fire_state.reload_secs - dt).max(0.0);
            continue;
        }

        // Step 5: empty mag → start a reload.
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

        // Step 6: ticking shot cooldown?
        if fire_state.cooldown_secs > 0.0 {
            fire_state.cooldown_secs = (fire_state.cooldown_secs - dt).max(0.0);
            continue;
        }

        // Step 7: fire one shot.
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

        // Resolve the hit against the target's snapshot armor.
        let (dealt, absorbed) =
            Resistances::resolve_hit(target_ballistic, penetration, raw_damage);

        // Mutate the shooter (drain mag, wear weapon).
        if let Some(weapon) = &mut loadout.0.primary {
            weapon.count = weapon.count.saturating_sub(1);
            weapon.degrade(1);
        }

        // Tracer.
        spawn_tracer(&mut commands, &tracer_assets, shooter_pos, target_pos);

        // Reset cooldown and queue the hit for the target pass.
        fire_state.cooldown_secs = if fire_rate > 0.0 { 1.0 / fire_rate } else { 1.0 };
        hits.push(HitIntent {
            target: target_entity,
            dealt,
            absorbed,
        });
    }

    // Pass 2: apply HP damage and armor wear via the targets query.
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

/// Spawn a tracer rectangle stretching from `from` to `to`.
fn spawn_tracer(commands: &mut Commands, assets: &TracerAssets, from: Vec2, to: Vec2) {
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
        Mesh2d(assets.mesh.clone()),
        MeshMaterial2d(assets.material.clone()),
        Transform {
            translation: Vec3::new(mid.x, mid.y, 0.6),
            rotation: Quat::from_rotation_z(angle),
            scale: Vec3::new(length, 1.0, 1.0),
        },
    ));
}

fn fade_tracers(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Tracer)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tracer) in &mut q {
        tracer.life_secs -= dt;
        if tracer.life_secs <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
