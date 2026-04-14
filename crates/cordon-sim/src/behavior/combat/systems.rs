//! Combat resolution: weapon firing, damage application.

use std::collections::HashMap;

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use cordon_core::item::{Item, ItemData, ItemDef, Loadout};
use cordon_core::primitive::{Health, Id, Pool, Resistances};
use cordon_data::gamedata::GameDataResource;

use super::components::{CombatTarget, FireState};
use super::events::{NpcPoolChanged, ShotFired};
use super::helpers::{equipped_ballistic, find_ammo_idx};
use crate::behavior::death::components::Dead;

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
    query: &Query<(Entity, &Transform, &Loadout), (With<Pool<Health>>, Without<Dead>)>,
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
pub fn resolve_combat(
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut shots: MessageWriter<ShotFired>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut sets: ParamSet<(
        // Read-only snapshot pass.
        Query<(Entity, &Transform, &Loadout), (With<Pool<Health>>, Without<Dead>)>,
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
        Query<&mut Pool<Health>, Without<Dead>>,
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

    // Pass 2: apply HP damage and emit one NpcPoolChanged per hit.
    // Capturing prev before deplete lets the effect dispatcher
    // detect threshold crossings (e.g. OnLowHealth) without having
    // to store its own previous-state tracking.
    let mut targets_apply = sets.p2();
    for hit in hits {
        if let Ok(mut hp) = targets_apply.get_mut(hit.target) {
            let prev = hp.current();
            hp.deplete(hit.dealt);
            pool_changed.write(NpcPoolChanged {
                entity: hit.target,
                pool: cordon_core::item::ResourceTarget::Health,
                prev,
                current: hp.current(),
                max: hp.max(),
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
