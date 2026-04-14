//! Combat resolution: weapon firing, damage application.

use std::collections::HashMap;

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use cordon_core::item::{Caliber, Item, ItemData, ItemDef, Loadout, ResourceTarget};
use cordon_core::primitive::{Health, Id, Pool, Resistances};
use cordon_data::gamedata::GameDataResource;

use super::components::{CombatTarget, FireState};
use super::events::{NpcPoolChanged, ShotFired};
use super::helpers::{equipped_ballistic, find_ammo_idx};
use crate::behavior::death::components::Dead;
use crate::entity::npc::NpcMarker;

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

/// Weapon stats resolved once per shooter per frame. The actual
/// per-shot damage is recomputed inside the shooter loop whenever
/// the active ammo box's def changes, so a shooter carrying mixed
/// ammo types gets correct damage numbers for each round (no
/// stale caching across box swaps).
struct WeaponStats {
    /// Caliber the shooter's weapon fires. Used to match ammo
    /// boxes in the pouch.
    caliber: Id<Caliber>,
    /// Seconds per shot (`1.0 / fire_rate`), precomputed once.
    period: f32,
    range: f32,
    /// Weapon's own bonus damage (long barrel, hand-tuned action),
    /// added to the active ammo's base damage.
    added_damage: u32,
    /// Target's ballistic resistance, snapshotted once per frame
    /// so every shot in the burst uses the same resistance value.
    target_ballistic: u32,
}

/// Result of simulating one shooter for one frame. Written back to
/// the shooter's components by the caller.
struct ShooterOutcome {
    cooldown: f32,
    /// Total rounds fired this frame, paired with the damage each
    /// one dealt. Length matches total shots; the `u32` on each
    /// entry is the resolved post-resistance damage.
    hits: Vec<u32>,
    hit_target: Option<Entity>,
    /// True when the NPC ran out of ammo mid-frame and should drop
    /// its combat target.
    stop_targeting: bool,
}

/// Resolve a shooter's equipped weapon stats into a flat
/// [`WeaponStats`]. Returns `None` when the shooter doesn't have a
/// fireable setup (no weapon, no matching ammo in the pouch,
/// invalid def); the caller treats that as "skip this frame".
///
/// This is a peek — it doesn't consume ammo. The shooter loop
/// resolves the active box's damage on demand.
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

    // Gate: must have at least one matching ammo box to fire this
    // frame. We don't compute damage here — that happens per shot.
    find_ammo_idx(loadout, &weapon.caliber, items)?;

    let period = if weapon.fire_rate > 0.0 {
        1.0 / weapon.fire_rate
    } else {
        1.0
    };

    Some(WeaponStats {
        caliber: weapon.caliber.clone(),
        period,
        range: weapon.range.value(),
        added_damage: weapon.added_damage,
        target_ballistic,
    })
}

/// Core frame simulation for a single shooter. Runs the fire →
/// catch-up loop against a shared `dt` budget and returns a
/// [`ShooterOutcome`] the caller can flush into components.
///
/// No magazine state — every shot pulls one round directly from
/// the first matching ammo box in the general pouch. When no
/// matching box remains, the shooter drops the target.
///
/// Damage is resolved per shot against the *active* ammo box's
/// def. We cache the last-seen box id so unchanged bursts skip
/// the HashMap lookup; only a box-swap forces a refresh.
fn simulate_shooter_frame(
    dt: f32,
    target: Entity,
    loadout: &mut Loadout,
    stats: &WeaponStats,
    initial_cooldown: f32,
    items: &HashMap<Id<Item>, ItemDef>,
) -> ShooterOutcome {
    let mut budget = dt;
    let mut cooldown = initial_cooldown;
    let mut hits: Vec<u32> = Vec::new();
    let mut stop_targeting = false;
    // Per-shot damage cache: `(ammo_def_id, resolved_damage)`.
    // Reset on box swap so mixed-caliber-same-pouch loadouts
    // produce correct numbers.
    let mut active: Option<(Id<Item>, u32)> = None;

    while budget > 0.0 {
        // --- Ammo check: is there any matching box left?
        let Some(ammo_idx) = find_ammo_idx(loadout, &stats.caliber, items) else {
            stop_targeting = true;
            break;
        };
        let ammo_box_id = loadout.general[ammo_idx].def_id.clone();

        // --- Resolve damage for the active box (cache hit on
        // same-box bursts, one lookup on swap). If the id doesn't
        // resolve (shouldn't happen — find_ammo_idx already
        // filtered) we bail out of the frame defensively.
        let dealt = match &active {
            Some((id, d)) if *id == ammo_box_id => *d,
            _ => {
                let Some(resolved) = resolve_ammo_damage(
                    &ammo_box_id,
                    items,
                    stats.added_damage,
                    stats.target_ballistic,
                ) else {
                    stop_targeting = true;
                    break;
                };
                active = Some((ammo_box_id.clone(), resolved));
                resolved
            }
        };

        // --- Firing phase: advance the cooldown, then fire as
        // many catch-up shots as the remaining budget affords.
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

        // Cap the burst at the rounds remaining *in the current
        // box* so a box-swap mid-burst triggers a fresh damage
        // resolve on the next outer-loop iteration.
        let box_rounds = loadout.general[ammo_idx].count;
        let to_fire = affordable.min(box_rounds);
        if to_fire == 0 {
            stop_targeting = true;
            break;
        }
        for _ in 0..to_fire {
            // Direct index decrement — we know ammo_idx is valid
            // for this burst because to_fire <= box_rounds.
            loadout.general[ammo_idx].count -= 1;
            hits.push(dealt);
        }
        // Clean up an emptied box; next loop iteration's
        // `find_ammo_idx` will pick the next matching box (or
        // terminate the burst).
        if loadout.general[ammo_idx].count == 0 {
            loadout.general.remove(ammo_idx);
            active = None;
        }

        // The first shot of the burst was already "paid for" by
        // exiting the cooldown branch — only the extras spend
        // additional budget.
        budget -= stats.period * (to_fire as f32 - 1.0);
        cooldown = stats.period;
    }

    let shots_fired = hits.len() as u32;
    ShooterOutcome {
        cooldown,
        hits,
        hit_target: if shots_fired > 0 { Some(target) } else { None },
        stop_targeting,
    }
}

/// Resolve an ammo def's damage against a target's ballistic
/// resistance. Returns `None` if the id doesn't point to an ammo
/// def (shouldn't happen in practice — `find_ammo_idx` already
/// filtered — but defensive).
fn resolve_ammo_damage(
    ammo_id: &Id<Item>,
    items: &HashMap<Id<Item>, ItemDef>,
    weapon_added: u32,
    target_ballistic: u32,
) -> Option<u32> {
    let def = items.get(ammo_id)?;
    let ItemData::Ammo(ammo) = &def.data else {
        return None;
    };
    let raw = ammo.damage + weapon_added;
    Some(Resistances::resolve_hit(
        target_ballistic,
        ammo.penetration,
        raw,
    ))
}

/// Tick down per-NPC fire cooldowns and apply damage when ready.
///
/// Orchestrator: three named phases, each expressed as a helper
/// function. The ParamSet plumbing stays here because it's
/// cross-phase; the per-phase logic reads top-to-bottom.
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
        Query<(Entity, &Transform, &Loadout), (With<NpcMarker>, Without<Dead>)>,
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

    // Phase 1: build a cheap snapshot keyed by entity so the
    // shooter loop doesn't hold a borrow on target components.
    let target_snapshot = snapshot_targets(&sets.p0(), items);

    // Phase 2: run each shooter's per-frame loop and collect hits.
    let hits = fire_shooters(&mut sets.p1(), &target_snapshot, items, dt, &mut shots);

    // Phase 3: apply accumulated hits to target HP pools.
    apply_hits(&mut sets.p2(), hits, &mut pool_changed);
}

/// Phase 1: snapshot every alive NPC's position + ballistic into
/// a HashMap keyed by entity. The shooter loop reads this without
/// holding a borrow on `Pool<Health>` or `Loadout`, which keeps
/// the ParamSet dance honest.
fn snapshot_targets(
    query: &Query<(Entity, &Transform, &Loadout), (With<NpcMarker>, Without<Dead>)>,
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

/// Phase 2: run each shooter's per-frame loop. Collects damage
/// into a `Vec<HitIntent>` rather than applying in place so the
/// apply pass can hold `&mut Pool<Health>` without fighting the
/// shooter's `&mut Loadout`. `ShotFired` is capped at one per
/// shooter per frame — at high `dt` (e.g. 64×) a 10 rps weapon
/// would otherwise spam the visual layer with identical tracers.
/// Damage still applies for every shot, so combat resolves
/// correctly.
fn fire_shooters(
    shooters: &mut Query<
        (
            Entity,
            &Transform,
            &mut CombatTarget,
            &mut FireState,
            &mut Loadout,
        ),
        Without<Dead>,
    >,
    target_snapshot: &HashMap<Entity, TargetInfo>,
    items: &HashMap<Id<Item>, ItemDef>,
    dt: f32,
    shots: &mut MessageWriter<ShotFired>,
) -> Vec<HitIntent> {
    let mut hits: Vec<HitIntent> = Vec::new();
    for (shooter_entity, shooter_transform, mut combat_target, mut fire_state, mut loadout) in
        shooters
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

        let outcome = simulate_shooter_frame(
            dt,
            target_entity,
            &mut loadout,
            &stats,
            fire_state.cooldown_secs,
            items,
        );
        fire_state.cooldown_secs = outcome.cooldown;

        if let Some(target) = outcome.hit_target {
            // Emit one tracer for the frame before the damage
            // pushes so the visual layer receives a single
            // "shooter shot" event even when the sim fired a burst.
            shots.write(ShotFired {
                shooter: shooter_entity,
                from: shooter_pos,
                to: target_info.pos,
            });
            // `outcome.hits` carries the per-shot resolved damage,
            // which may differ across shots in a burst if the
            // shooter's ammo box swapped mid-frame.
            for dealt in outcome.hits {
                hits.push(HitIntent { target, dealt });
            }
        }

        if outcome.stop_targeting {
            combat_target.0 = None;
            *fire_state = FireState::default();
        }
    }
    hits
}

/// Phase 3: apply each queued hit and emit one [`NpcPoolChanged`]
/// per damage instance. Capturing `prev` before `deplete` lets the
/// effect dispatcher detect threshold crossings (e.g. `OnLowHealth`)
/// without storing its own previous-state tracking.
fn apply_hits(
    targets: &mut Query<&mut Pool<Health>, Without<Dead>>,
    hits: Vec<HitIntent>,
    pool_changed: &mut MessageWriter<NpcPoolChanged>,
) {
    for hit in hits {
        if let Ok(mut hp) = targets.get_mut(hit.target) {
            let prev = hp.current();
            hp.deplete(hit.dealt);
            pool_changed.write(NpcPoolChanged {
                entity: hit.target,
                pool: ResourceTarget::Health,
                prev,
                current: hp.current(),
                max: hp.max(),
            });
        }
    }
}
