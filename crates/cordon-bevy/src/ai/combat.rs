//! Combat: vision, engagement, damage, death, looting.
//!
//! Each NPC has a [`Vision`] radius. [`update_engagement`] scans for
//! hostile NPCs in vision; if a hostile is in weapon range with clear
//! line-of-sight, it pushes [`Action::Engage`]. Otherwise it pushes
//! [`Action::Walk`] toward the target. Damage is resolved each tick by
//! [`resolve_engage_actions`] using the loadout's equipped weapon and
//! the first matching ammo box from the general pouch.
//!
//! Anomaly disks block line-of-sight via [`AnomalyZone`].
//!
//! Death: when an NPC's health hits zero, [`handle_deaths`] tags the
//! entity with [`Dead`] and recolors the dot. [`cleanup_corpses`]
//! despawns it after [`CORPSE_PERSISTENCE_MINUTES`] of game time, or
//! immediately when the loadout is fully looted.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::entity::npc::Npc;
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{GameTime, Id, Rank, Resistances, Uid};
use cordon_data::gamedata::GameDataResource;
use moonshine_behavior::prelude::*;

use super::behavior::Action;
use crate::PlayingState;
use crate::laptop::{MapWorldEntity, NpcDot, NpcFaction};
use crate::world::SimWorld;

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 25 base + 3 per rank tier above Novice + 5 if
    /// the NPC's faction has military training.
    pub fn for_npc(rank: Rank, is_military: bool) -> Self {
        let from_rank = 25.0 + (rank.tier() as f32 - 1.0) * 3.0;
        let from_faction = if is_military { 5.0 } else { 0.0 };
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

/// Marker for a corpse with its time of death.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
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

/// How long corpses persist before despawn (7 in-game days).
pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

/// How fast a single loot transfer takes (seconds per item).
const LOOT_INTERVAL_SECS: f32 = 0.4;

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

/// Time between shots (s) for the equipped weapon.
fn shot_cooldown(items: &HashMap<Id<Item>, ItemDef>, loadout: &Loadout) -> f32 {
    let Some(inst) = loadout.equipped_weapon() else {
        return f32::INFINITY;
    };
    let Some(def) = items.get(&inst.def_id) else {
        return f32::INFINITY;
    };
    match &def.data {
        ItemData::Weapon(w) if w.fire_rate > 0.0 => 1.0 / w.fire_rate,
        _ => f32::INFINITY,
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

/// Plugin registering combat systems.
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_engagement,
                resolve_engage_actions,
                fade_tracers,
                try_start_looting,
                drive_loot_actions,
                handle_deaths,
                cleanup_corpses,
            )
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Distance under which an alive NPC will start looting an adjacent corpse.
const LOOT_REACH: f32 = 8.0;

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
            // In range: enter or stay in Engage.
            let already_engaging = matches!(
                behavior.current(),
                Action::Engage { target, .. } if *target == target_uid
            );
            if !already_engaging {
                let cooldown = shot_cooldown(items, &npc.loadout);
                let _ = behavior.try_start(Action::Engage {
                    target: target_uid,
                    cooldown_secs: cooldown,
                });
            }
        } else {
            // Out of range: walk toward the target.
            let already_walking = matches!(behavior.current(), Action::Walk { .. });
            if !already_walking {
                let _ = behavior.try_start(Action::Walk {
                    target: target_pos,
                    speed: 12.0,
                });
            }
        }
    }
}

/// Tick down engage cooldowns and apply damage when ready.
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

    // Snapshot positions of all alive NPCs so the shooter loop can look
    // up its target's location and we can spawn tracers later.
    let positions: HashMap<Uid<Npc>, Vec2> = targets
        .iter()
        .map(|(dot, t)| (dot.uid, t.translation.truncate()))
        .collect();

    // (shooter_dot, target_dot, dealt, absorbed, shooter_pos, target_pos)
    let mut hits: Vec<(NpcDot, NpcDot, u32, u32, Vec2, Vec2)> = Vec::new();

    for (shooter_dot, shooter_transform, mut behavior) in &mut shooters {
        // Tick the cooldown if we're engaging.
        let (target_uid, ready) = match behavior.current_mut() {
            Action::Engage {
                target,
                cooldown_secs,
            } => {
                *cooldown_secs -= dt;
                let ready = *cooldown_secs <= 0.0;
                if ready {
                    *cooldown_secs = 0.0;
                }
                (*target, ready)
            }
            _ => continue,
        };
        if !ready {
            continue;
        }

        let Some(&target_pos) = positions.get(&target_uid) else {
            continue;
        };
        let shooter_pos = shooter_transform.translation.truncate();
        let target_dot = NpcDot { uid: target_uid };

        // Look up shooter weapon, ammo, and target ballistic in one borrow.
        let Some(shooter) = sim.0.npcs.get(&shooter_dot.uid) else {
            continue;
        };
        let Some(weapon_inst) = shooter.loadout.equipped_weapon() else {
            continue;
        };
        let Some(weapon_def) = items.get(&weapon_inst.def_id) else {
            continue;
        };
        let (caliber, weapon_added) = match &weapon_def.data {
            ItemData::Weapon(w) => (w.caliber.clone(), w.added_damage),
            _ => continue,
        };
        let Some(ammo_idx) = find_ammo_idx(&shooter.loadout, &caliber, items) else {
            continue;
        };
        let ammo_inst = &shooter.loadout.general[ammo_idx];
        let Some(ammo_def) = items.get(&ammo_inst.def_id) else {
            continue;
        };
        let (ammo_damage, penetration) = match &ammo_def.data {
            ItemData::Ammo(a) => (a.damage, a.penetration),
            _ => continue,
        };
        let raw_damage = ammo_damage + weapon_added;

        let Some(target) = sim.0.npcs.get(&target_dot.uid) else {
            continue;
        };
        let target_ballistic = equipped_ballistic(&target.loadout, items);
        let (dealt, absorbed) =
            Resistances::resolve_hit(target_ballistic, penetration, raw_damage);

        // Reset shooter cooldown to a fresh interval.
        if let Action::Engage { cooldown_secs, .. } = behavior.current_mut() {
            *cooldown_secs = match &weapon_def.data {
                ItemData::Weapon(w) if w.fire_rate > 0.0 => 1.0 / w.fire_rate,
                _ => 1.0,
            };
        }

        hits.push((
            *shooter_dot,
            target_dot,
            dealt,
            absorbed,
            shooter_pos,
            target_pos,
        ));
    }

    // Spawn tracers for every shot fired this tick.
    let tracer_color = Color::srgba(1.0, 0.92, 0.55, 0.95);
    for (_, _, _, _, from, to) in &hits {
        spawn_tracer(
            &mut commands,
            &mut meshes,
            &mut materials,
            tracer_color,
            *from,
            *to,
        );
    }

    // Apply mutations.
    for (shooter_dot, target_dot, dealt, absorbed, _, _) in hits {
        if let Some(shooter) = sim.0.npcs.get_mut(&shooter_dot.uid) {
            // Decrement ammo on the shooter.
            let weapon_caliber = shooter
                .loadout
                .equipped_weapon()
                .and_then(|w| items.get(&w.def_id))
                .and_then(|def| match &def.data {
                    ItemData::Weapon(w) => Some(w.caliber.clone()),
                    _ => None,
                });
            if let Some(caliber) = weapon_caliber
                && let Some(idx) = find_ammo_idx(&shooter.loadout, &caliber, items)
            {
                let ammo = &mut shooter.loadout.general[idx];
                ammo.count = ammo.count.saturating_sub(1);
                if ammo.count == 0 {
                    shooter.loadout.general.remove(idx);
                }
            }
            // Weapons take 1 durability per shot.
            if let Some(weapon) = &mut shooter.loadout.primary {
                weapon.degrade(1);
            }
        }

        if let Some(target) = sim.0.npcs.get_mut(&target_dot.uid) {
            target.health.damage(dealt);
            if absorbed > 0
                && let Some(armor) = &mut target.loadout.armor
            {
                armor.degrade(absorbed);
            }
        }
    }
}

/// Push `Action::Loot` for alive NPCs that are standing near a corpse
/// and not currently engaged in combat.
#[allow(clippy::type_complexity)]
fn try_start_looting(
    sim: Option<Res<SimWorld>>,
    corpses: Query<(&NpcDot, &Transform), With<Dead>>,
    mut alive: Query<
        (&NpcDot, &Transform, BehaviorMut<Action>),
        Without<Dead>,
    >,
) {
    let Some(sim) = sim else { return };

    // Snapshot non-empty corpses with their positions.
    let corpse_snapshot: Vec<(Uid<Npc>, Vec2)> = corpses
        .iter()
        .filter_map(|(dot, t)| {
            let npc = sim.0.npcs.get(&dot.uid)?;
            if npc.loadout.is_empty() {
                return None;
            }
            Some((dot.uid, t.translation.truncate()))
        })
        .collect();
    if corpse_snapshot.is_empty() {
        return;
    }

    for (looter_dot, transform, mut behavior) in &mut alive {
        // Don't pre-empt fighting.
        if matches!(
            behavior.current(),
            Action::Engage { .. } | Action::Loot { .. } | Action::Flee { .. }
        ) {
            continue;
        }
        let pos = transform.translation.truncate();
        let nearest = corpse_snapshot
            .iter()
            .filter(|(uid, _)| *uid != looter_dot.uid)
            .min_by(|(_, a), (_, b)| {
                pos.distance_squared(*a)
                    .partial_cmp(&pos.distance_squared(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some((corpse_uid, corpse_pos)) = nearest else {
            continue;
        };
        if pos.distance(*corpse_pos) > LOOT_REACH {
            continue;
        }
        let _ = behavior.try_start(Action::Loot {
            target: *corpse_uid,
            progress_secs: LOOT_INTERVAL_SECS,
        });
    }
}

/// Drive `Action::Loot`: tick the progress timer, transfer items.
#[allow(clippy::type_complexity)]
fn drive_loot_actions(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    mut q: Query<(&NpcDot, BehaviorMut<Action>), Without<Dead>>,
    corpses: Query<&NpcDot, With<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let dt = time.delta_secs();

    let corpse_uids: HashMap<Uid<Npc>, ()> =
        corpses.iter().map(|dot| (dot.uid, ())).collect();

    let mut transfers: Vec<(NpcDot, NpcDot)> = Vec::new();

    for (looter_dot, mut behavior) in &mut q {
        let target_uid = match behavior.current_mut() {
            Action::Loot {
                target,
                progress_secs,
            } => {
                *progress_secs -= dt;
                if *progress_secs > 0.0 {
                    continue;
                }
                *progress_secs = LOOT_INTERVAL_SECS;
                *target
            }
            _ => continue,
        };

        if !corpse_uids.contains_key(&target_uid) {
            // Corpse vanished — bail out of looting.
            let _ = behavior.try_start(Action::Idle { timer: 0.5 });
            continue;
        }
        let corpse_dot = NpcDot { uid: target_uid };
        transfers.push((*looter_dot, corpse_dot));
    }

    for (looter_dot, corpse_dot) in transfers {
        // Pull one item from the corpse into the looter (if room).
        // Borrow each NPC sequentially to avoid double-mut.
        let item_taken = {
            let Some(corpse) = sim.0.npcs.get_mut(&corpse_dot.uid) else {
                continue;
            };
            // Pop in priority order: primary, secondary, helmet, armor,
            // relics, then general items.
            corpse
                .loadout
                .primary
                .take()
                .or_else(|| corpse.loadout.secondary.take())
                .or_else(|| corpse.loadout.helmet.take())
                .or_else(|| corpse.loadout.armor.take())
                .or_else(|| corpse.loadout.relics.pop())
                .or_else(|| corpse.loadout.general.pop())
        };
        let Some(item) = item_taken else { continue };

        if let Some(looter) = sim.0.npcs.get_mut(&looter_dot.uid) {
            // Use a generous capacity since archetypes might not have
            // populated armor data; default cap = 20 for now.
            let capacity = looter.loadout.general.len() as u8 + 1;
            let _ = looter
                .loadout
                .add_to_general(item, capacity.max(20));
        }
    }
}

/// Tag NPCs whose health hit zero as dead and replace their dot with
/// an X-shaped mesh.
fn handle_deaths(
    sim: Option<Res<SimWorld>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q: Query<(Entity, &NpcDot, &MeshMaterial2d<ColorMaterial>), Without<Dead>>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, npc_dot, mat_handle) in &q {
        let Some(npc) = sim.0.npcs.get(&npc_dot.uid) else {
            continue;
        };
        if npc.health.is_alive() {
            continue;
        }
        commands.entity(entity).insert(Dead { died_at: now });

        // Hide the original circle by recoloring it transparent.
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.color = Color::NONE;
        }

        // Spawn two crossed bar children to form the X.
        let bar_color = Color::srgba(0.55, 0.1, 0.1, 0.95);
        let bar_mesh = meshes.add(Rectangle::new(10.0, 1.5));
        let bar_mat = materials.add(ColorMaterial::from_color(bar_color));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh2d(bar_mesh.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ));
            parent.spawn((
                Mesh2d(bar_mesh.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4)),
            ));
        });
    }
}

/// Despawn corpses after the persistence window or once their loadout
/// has been fully looted.
fn cleanup_corpses(
    sim: Option<Res<SimWorld>>,
    mut commands: Commands,
    q: Query<(Entity, &Dead, &NpcDot)>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, dead, npc_dot) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = sim
            .0
            .npcs
            .get(&npc_dot.uid)
            .map(|n| n.loadout.is_empty())
            .unwrap_or(true);
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            commands.entity(entity).despawn();
        }
    }
}

/// Convert a [`GameTime`] to absolute minutes since day 1, 00:00.
fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

/// Game-minutes elapsed between two times. Saturating; never negative.
fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
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
