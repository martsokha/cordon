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
use crate::components::Hp;
use crate::plugin::SimSet;

/// A weapon discharged from `from` toward `to`. The visual layer
/// renders a tracer; the audio layer plays a gunshot. Emitted
/// at most once per shooter per frame by `resolve_combat`.
#[derive(Message, Debug, Clone, Copy)]
pub struct ShotFired {
    pub shooter: Entity,
    pub from: Vec2,
    pub to: Vec2,
}

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
fn equipped_ballistic(loadout: &Loadout, items: &HashMap<Id<Item>, ItemDef>) -> u32 {
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
        app.add_message::<NpcDamaged>();
        app.add_systems(Update, resolve_combat.in_set(SimSet::Combat));
    }
}

/// Emitted by [`resolve_combat`] for each hit that applied damage.
///
/// Carries `prev_hp` so downstream systems (effect dispatcher,
/// death handler) can detect HP crossings without needing their
/// own previous-state tracking. Fires once per hit, not per
/// shooter — a burst that lands 5 hits in one frame produces
/// 5 messages.
#[derive(Message, Debug, Clone, Copy)]
pub struct NpcDamaged {
    /// The entity that took damage.
    pub target: Entity,
    /// Amount actually applied via `hp.deplete` (which saturates
    /// at 0 — so this is `min(dealt, prev_hp)`).
    pub dealt: u32,
    /// HP immediately before the depletion. Let subscribers
    /// compute ratios (`prev / max`) or detect crossings
    /// (`prev > threshold && new <= threshold`).
    pub prev_hp: u32,
}

/// One pending hit produced by the shooter loop and applied to the
/// target in a separate pass. Decoupling lets us hold one mutable
/// query at a time, avoiding overlapping `&mut Loadout` access.
struct HitIntent {
    target: Entity,
    dealt: u32,
}

/// Snapshot of a potential target built before the shooter loop.
/// Captures just enough state to resolve a hit without holding a
/// borrow on the target's components — the shooter loop needs
/// mutable access to `Loadout` which would overlap otherwise.
#[derive(Clone, Copy)]
struct TargetInfo {
    pos: Vec2,
    ballistic: u32,
}

/// Weapon + ammo stats resolved from a shooter's current loadout.
/// Pulled out into its own struct so `simulate_shooter_frame` has
/// a flat, boring argument list instead of a wall of locals.
struct WeaponStats {
    caliber: Id<cordon_core::item::Caliber>,
    magazine: u32,
    /// Seconds per shot (`1.0 / fire_rate`), precomputed once.
    period: f32,
    range: f32,
    /// Damage dealt *after* target ballistic resistance has already
    /// been subtracted — precomputed so the shooter loop doesn't
    /// repeat the resolve for every catch-up shot.
    dealt: u32,
}

/// Result of simulating one shooter for one frame. Written back to
/// the shooter's components by the caller.
struct ShooterOutcome {
    cooldown: f32,
    shots_fired: u32,
    hit_target: Option<Entity>,
    /// True when the NPC ran out of ammo mid-frame and should drop
    /// its combat target.
    stop_targeting: bool,
}

/// Build the per-target snapshot used by the shooter loop so
/// shooters don't need to query target components mutably.
fn build_target_snapshot(
    query: &Query<(Entity, &Transform, &Loadout), (With<Hp>, Without<Dead>)>,
    items: &HashMap<Id<Item>, ItemDef>,
) -> HashMap<Entity, TargetInfo> {
    let mut m = HashMap::with_capacity(1024);
    for (entity, transform, loadout) in query.iter() {
        let pos = transform.translation.truncate();
        let ballistic = equipped_ballistic(loadout, items);
        m.insert(entity, TargetInfo { pos, ballistic });
    }
    m
}

/// Resolve a shooter's equipped weapon + loaded-ammo stats into a
/// flat [`WeaponStats`] + precomputed damage. Returns `None` when
/// the shooter doesn't have a fireable setup (no weapon, no ammo,
/// invalid def, etc.); the caller treats that as "skip this frame".
fn load_weapon_stats(
    loadout: &Loadout,
    target_ballistic: u32,
    items: &HashMap<Id<Item>, ItemDef>,
) -> Option<WeaponStats> {
    let weapon_inst = loadout.equipped_weapon()?;
    let weapon_def = items.get(&weapon_inst.def_id)?;
    let ItemData::Weapon(weapon) = &weapon_def.data else {
        return None;
    };

    let loaded_ammo_id = weapon_inst.loaded_ammo.clone()?;
    let ammo_def = items.get(&loaded_ammo_id)?;
    let ItemData::Ammo(ammo) = &ammo_def.data else {
        return None;
    };

    let raw_damage = ammo.damage + weapon.added_damage;
    let dealt = Resistances::resolve_hit(target_ballistic, ammo.penetration, raw_damage);

    let period = if weapon.fire_rate > 0.0 {
        1.0 / weapon.fire_rate
    } else {
        1.0
    };

    Some(WeaponStats {
        caliber: weapon.caliber.clone(),
        magazine: weapon.magazine,
        period,
        range: weapon.range.value(),
        dealt,
    })
}

/// Core frame simulation for a single shooter. Runs the fire →
/// catch-up loop against a shared `dt` budget and returns a
/// [`ShooterOutcome`] the caller can flush into components.
///
/// This is the interesting part of combat — split out of
/// `resolve_combat` so the outer system is mostly plumbing. The
/// `loadout` reference is mutable because mag refills drain ammo
/// pouches; everything else is local state. Fire tempo is
/// controlled entirely by `WeaponStats::period`: when a mag runs
/// dry the loop tops it up in place from the general pouch and
/// keeps firing within the same `dt` budget.
fn simulate_shooter_frame(
    dt: f32,
    target: Entity,
    loadout: &mut Loadout,
    stats: &WeaponStats,
    initial_cooldown: f32,
    initial_mag: u32,
    items: &HashMap<Id<Item>, ItemDef>,
) -> ShooterOutcome {
    let mut budget = dt;
    let mut cooldown = initial_cooldown;
    let mut mag_live = initial_mag;
    let mut shots_fired: u32 = 0;
    let mut stop_targeting = false;

    while budget > 0.0 {
        // --- Empty-mag phase: refill instantly from the general
        // pouch if we can, otherwise drop the target.
        if mag_live == 0 {
            let refilled = refill_magazine(loadout, items, &stats.caliber, stats.magazine);
            if refilled {
                mag_live = loadout.primary.as_ref().map(|w| w.count).unwrap_or(0);
                continue;
            }
            // No ammo pouches left → give up. Caller will drop
            // the combat target so the AI can pick something else.
            stop_targeting = true;
            break;
        }

        // --- Firing phase: advance the cooldown, then fire as
        // many catch-up shots as the remaining budget can afford.
        if cooldown > 0.0 {
            let consumed = cooldown.min(budget);
            cooldown -= consumed;
            budget -= consumed;
            if cooldown > 0.0 {
                break;
            }
        }
        let affordable = if stats.period > 0.0 {
            1 + (budget / stats.period).floor() as u32
        } else {
            1
        };
        let to_fire = affordable.min(mag_live);
        if to_fire == 0 {
            // Shouldn't be reachable given the branches above, but
            // guard to prevent an infinite loop on weird input.
            break;
        }
        for _ in 0..to_fire {
            if let Some(weapon) = &mut loadout.primary {
                weapon.count = weapon.count.saturating_sub(1);
            }
        }
        shots_fired += to_fire;
        mag_live = mag_live.saturating_sub(to_fire);
        // The first shot of the burst was already "paid for" by
        // exiting the cooldown branch — only the extras spend
        // additional budget.
        budget -= stats.period * (to_fire as f32 - 1.0);
        cooldown = stats.period;
    }

    ShooterOutcome {
        cooldown,
        shots_fired,
        hit_target: if shots_fired > 0 { Some(target) } else { None },
        stop_targeting,
    }
}

/// Tick down per-NPC fire cooldowns and apply damage when ready.
///
/// Structured as four small phases — snapshot targets, run each
/// shooter's per-frame loop, emit one tracer per shooter, apply
/// damage — so the big ParamSet plumbing stays at the top and the
/// per-shooter logic lives in [`simulate_shooter_frame`].
///
/// All NPC mutable access flows through a [`ParamSet`] so multiple
/// `&mut Loadout` queries don't overlap.
fn resolve_combat(
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut shots: MessageWriter<ShotFired>,
    mut damaged: MessageWriter<NpcDamaged>,
    mut sets: ParamSet<(
        // Read-only snapshot pass.
        Query<(Entity, &Transform, &Loadout), (With<Hp>, Without<Dead>)>,
        // Shooter mutation pass.
        Query<
            (
                Entity,
                &Transform,
                &mut CombatTarget,
                &mut FireState,
                &mut Loadout,
            ),
            Without<Dead>,
        >,
        // Target apply pass.
        Query<&mut Hp, Without<Dead>>,
    )>,
) {
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    // Pass 0: snapshot every alive NPC's position + ballistic.
    let target_snapshot = build_target_snapshot(&sets.p0(), items);

    // Pass 1: run each shooter's per-frame loop, collecting hits
    // and tracer events. `ShotFired` is capped at one per shooter
    // per frame — at 64× dt a 10 rps weapon would otherwise spam
    // the visual layer with ~10 identical tracers, which both
    // crushes the renderer and looks bad. Damage still applies
    // for every shot, so combat resolves correctly.
    let mut hits: Vec<HitIntent> = Vec::new();
    {
        let mut shooters = sets.p1();
        for (shooter_entity, shooter_transform, mut combat_target, mut fire_state, mut loadout) in
            &mut shooters
        {
            let Some(target_entity) = combat_target.0 else {
                continue;
            };
            let Some(&target_info) = target_snapshot.get(&target_entity) else {
                combat_target.0 = None;
                *fire_state = FireState::default();
                continue;
            };
            let Some(stats) = load_weapon_stats(&loadout, target_info.ballistic, items) else {
                combat_target.0 = None;
                *fire_state = FireState::default();
                continue;
            };

            let shooter_pos = shooter_transform.translation.truncate();
            if shooter_pos.distance(target_info.pos) > stats.range {
                continue;
            }

            let mag_count = loadout.equipped_weapon().map(|w| w.count).unwrap_or(0);

            let outcome = simulate_shooter_frame(
                dt,
                target_entity,
                &mut loadout,
                &stats,
                fire_state.cooldown_secs,
                mag_count,
                items,
            );

            fire_state.cooldown_secs = outcome.cooldown;

            if let Some(target) = outcome.hit_target {
                // One tracer per shooter per frame (see comment
                // above); emit before the damage push so the
                // visual layer receives a single "shooter shot"
                // event even when the sim fired a burst.
                shots.write(ShotFired {
                    shooter: shooter_entity,
                    from: shooter_pos,
                    to: target_info.pos,
                });
                for _ in 0..outcome.shots_fired {
                    hits.push(HitIntent {
                        target,
                        dealt: stats.dealt,
                    });
                }
            }

            if outcome.stop_targeting {
                combat_target.0 = None;
                *fire_state = FireState::default();
            }
        }
    }

    // Pass 2: apply HP damage and emit one NpcDamaged per hit.
    // Capturing prev_hp before deplete lets the effect dispatcher
    // detect HP-crossing triggers (e.g. OnHpLow) without having
    // to store its own prev-hp state anywhere.
    let mut targets_apply = sets.p2();
    for hit in hits {
        if let Ok(mut hp) = targets_apply.get_mut(hit.target) {
            let prev_hp = hp.current();
            hp.deplete(hit.dealt);
            let dealt = prev_hp.saturating_sub(hp.current());
            damaged.write(NpcDamaged {
                target: hit.target,
                dealt,
                prev_hp,
            });
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
