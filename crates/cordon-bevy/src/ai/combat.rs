//! Combat: vision, weapon firing, damage application, tracers.
//!
//! Engagement *decisions* (which target, when to advance) live in
//! [`super::squad`]. This module owns the per-NPC firing loop:
//! reading the [`CombatTarget`] component the squad system wrote,
//! ticking the [`FireState`] cooldowns, applying damage when ready,
//! and spawning tracers.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::entity::npc::Npc;
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Id, Rank, Resistances, Uid};
use cordon_data::gamedata::GameDataResource;

use super::AiSet;
use super::behavior::{CombatTarget, FireState};
use super::death::Dead;
use crate::PlayingState;
use crate::laptop::{MapWorldEntity, NpcDot};
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

/// Plugin registering combat systems (firing, damage, tracers).
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

/// Tick down per-NPC fire cooldowns and apply damage when ready.
///
/// Runs after the squad layer has decided who each member is firing
/// at (via [`CombatTarget`]). For every NPC with a target:
///   1. If the target died/vanished, clear the target and skip.
///   2. If the target is out of weapon range, skip (squad will move us).
///   3. Tick `reload_secs`. If still reloading, skip.
///   4. If the magazine is empty, start a reload (refill from pouch).
///   5. Tick `cooldown_secs`. If still ticking, skip.
///   6. Otherwise: fire one shot, drain a round, wear the weapon,
///      apply HP damage and armor wear to the target, spawn a tracer,
///      reset the cooldown.
#[allow(clippy::type_complexity)]
fn resolve_combat(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    tracer_assets: Res<TracerAssets>,
    mut commands: Commands,
    mut shooters: Query<
        (&NpcDot, &Transform, &mut CombatTarget, &mut FireState),
        Without<Dead>,
    >,
    targets: Query<(&NpcDot, &Transform), Without<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    // Snapshot positions of all alive NPCs for target lookups.
    let positions: HashMap<Uid<Npc>, Vec2> = targets
        .iter()
        .map(|(dot, t)| (dot.uid, t.translation.truncate()))
        .collect();

    for (shooter_dot, shooter_transform, mut combat_target, mut fire_state) in &mut shooters {
        // Step 0: do we even have a target?
        let Some(target_uid) = combat_target.0 else {
            continue;
        };

        // Step 1: target still alive?
        let Some(target_pos) = positions.get(&target_uid).copied() else {
            // Target despawned — drop combat.
            combat_target.0 = None;
            *fire_state = FireState::default();
            continue;
        };

        // Step 2: read weapon stats and current magazine.
        let Some(shooter_npc) = sim.0.npcs.get(&shooter_dot.uid) else {
            continue;
        };
        let Some(weapon_inst) = shooter_npc.loadout.equipped_weapon() else {
            // Broken or missing primary: drop combat.
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

        // Step 3: out of range? Don't fire (squad layer is moving us).
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
            let started =
                refill_magazine(&mut sim, &shooter_dot, items, &caliber, magazine);
            if started {
                fire_state.reload_secs = reload_secs_def;
            } else {
                // No matching ammo → drop combat.
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
            // Mag has rounds but no recorded type — bail and let the
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

        // Resolve the hit against the target's armor.
        let target_ballistic = sim
            .0
            .npcs
            .get(&target_uid)
            .map(|npc| equipped_ballistic(&npc.loadout, items))
            .unwrap_or(0);
        let (dealt, absorbed) =
            Resistances::resolve_hit(target_ballistic, penetration, raw_damage);

        // Apply mutations to shooter (drain a round, wear the weapon).
        if let Some(shooter_mut) = sim.0.npcs.get_mut(&shooter_dot.uid)
            && let Some(weapon) = &mut shooter_mut.loadout.primary
        {
            weapon.count = weapon.count.saturating_sub(1);
            weapon.degrade(1);
        }
        // Apply mutations to target (HP damage, armor wear).
        if let Some(target_mut) = sim.0.npcs.get_mut(&target_uid) {
            target_mut.health.damage(dealt);
            if absorbed > 0
                && let Some(armor) = &mut target_mut.loadout.armor
            {
                armor.degrade(absorbed);
            }
        }

        // Visual: spawn the tracer.
        spawn_tracer(&mut commands, &tracer_assets, shooter_pos, target_pos);

        // Reset the shot cooldown.
        fire_state.cooldown_secs = if fire_rate > 0.0 { 1.0 / fire_rate } else { 1.0 };
    }
}

/// Pull one matching ammo box from the shooter's general pouch and
/// refill the primary weapon up to its magazine size.
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

/// Despawn expired tracers.
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
