//! Effect dispatcher.
//!
//! Drives [`TimedEffect`] and [`TriggeredEffect`] at runtime.
//! Connects three inputs to one per-entity state
//! ([`ActiveEffects`]) and one global scheduler
//! ([`PeriodicTriggers`]):
//!
//! - [`NpcDamaged`] messages from combat fire `OnHit` and edge-
//!   triggered `OnHpLow` relic effects.
//! - Loadout changes (add/remove relic) register or prune
//!   periodic entries.
//! - The game clock's minute rollover ticks every active
//!   effect and fires eligible periodic entries.
//!
//! Runs in [`SimSet::Effects`] between combat and death so an
//! `OnHpLow` heal can save a carrier in the same frame combat
//! depleted them.
//!
//! # What each `TimedEffect::target` does here
//!
//! | Target | Handler |
//! |---|---|
//! | `Health` | `hp.restore` for positive, `hp.deplete` for negative. Positive clamps at max. |
//! | `Damage` | `hp.deplete(value as u32)`. Distinct from `Health` by authorial intent. |
//! | `Stamina` | `stamina.restore` / `deplete`. |
//! | `Hunger`  | `hunger.restore` / `deplete`. |
//! | `Corruption` | `corruption.restore` for positive (gain corruption), `deplete` for negative (scrubbing). |
//!
//! `Bleeding`, `Poison`, and `Smoke` were deleted from
//! [`ResourceTarget`](cordon_core::item::ResourceTarget)
//! before this commit — status flags and area regions need
//! different data shapes and will come back when their
//! respective subsystems land.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::item::{EffectTrigger, Item, ItemData, Loadout, ResourceTarget, TimedEffect};
use cordon_core::primitive::{GameTime, Id};
use cordon_data::gamedata::GameDataResource;

use crate::behavior::Dead;
use crate::combat::NpcDamaged;
use crate::components::{
    ActiveEffect, ActiveEffectSource, ActiveEffects, Hp, HungerPool, NpcMarker, CorruptionPool,
    StaminaPool,
};
use crate::plugin::SimSet;
use crate::resources::GameClock;

/// Plugin registering the effect dispatcher.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PeriodicTriggers>();
        app.add_systems(
            Update,
            (
                sync_periodic_triggers,
                dispatch_damage_triggers,
                fire_periodic_triggers,
                tick_active_effects,
            )
                .chain()
                .in_set(SimSet::Effects),
        );
    }
}

/// Scheduled `Periodic` relic triggers across every NPC.
///
/// A single global resource rather than per-entity state
/// because the table is small (population × relic slots ≈
/// hundreds of entries max at steady state) and a plain
/// `Vec<Entry>` is easier to reason about than a component
/// with insertion / removal on loadout change.
#[derive(Resource, Debug, Default)]
pub struct PeriodicTriggers {
    entries: Vec<PeriodicEntry>,
}

/// One scheduled periodic trigger.
#[derive(Debug, Clone)]
struct PeriodicEntry {
    /// Which entity carries the relic.
    entity: Entity,
    /// Which relic produced this trigger. Used to prune on
    /// loadout change.
    relic: Id<Item>,
    /// The effect to apply on each fire.
    effect: TimedEffect,
    /// How often the trigger fires. Stored so the re-schedule
    /// step can compute the next-fire time.
    period_minutes: u32,
    /// When the next fire is scheduled. Compared against
    /// `GameClock::now` each tick.
    next_fire: GameTime,
}

/// Read [`NpcDamaged`] messages and fire matching `OnHit` and
/// edge-triggered `OnHpLow` effects from the target's equipped
/// relics.
///
/// Runs before [`tick_active_effects`] so instant-effect heals
/// land the same frame the damage was applied — which is why
/// an `OnHpLow` heal can save a carrier that just hit zero HP.
/// The query filters `Without<Dead>` because dead entities
/// don't get new triggers (the frame's already resolved).
fn dispatch_damage_triggers(
    mut damaged: MessageReader<NpcDamaged>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut targets: Query<
        (
            &Loadout,
            &mut ActiveEffects,
            &mut Hp,
            &mut StaminaPool,
            &mut HungerPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    let items = &data.0.items;

    for event in damaged.read() {
        let Ok((loadout, mut active, mut hp, mut stamina, mut hunger, mut corruption)) =
            targets.get_mut(event.target)
        else {
            continue;
        };

        for relic_instance in &loadout.relics {
            let Some(def) = items.get(&relic_instance.def_id) else {
                continue;
            };
            let ItemData::Relic(relic) = &def.data else {
                continue;
            };
            for triggered in &relic.triggered {
                match &triggered.trigger {
                    EffectTrigger::OnHit => {
                        apply_or_queue(
                            triggered.effect,
                            ActiveEffectSource::OneShot,
                            now,
                            &mut active,
                            &mut hp,
                            &mut stamina,
                            &mut hunger,
                            &mut corruption,
                        );
                    }
                    EffectTrigger::OnHpLow(ratio) => {
                        // Edge trigger: fired only on the frame
                        // HP crosses below `max * ratio`.
                        let max = hp.max() as f32;
                        if max <= 0.0 {
                            continue;
                        }
                        let threshold = max * ratio;
                        let was_above = (event.prev_hp as f32) > threshold;
                        let now_below = (hp.current() as f32) <= threshold;
                        if was_above && now_below {
                            apply_or_queue(
                                triggered.effect,
                                ActiveEffectSource::OneShot,
                                now,
                                &mut active,
                                &mut hp,
                                &mut stamina,
                                &mut hunger,
                                &mut corruption,
                            );
                        }
                    }
                    EffectTrigger::Periodic(_) => {
                        // Periodic effects are scheduled by
                        // `sync_periodic_triggers`; this path
                        // doesn't fire them.
                    }
                }
            }
        }
    }
}

/// Walk every entity with a changed loadout and rebuild its
/// periodic-trigger entries in [`PeriodicTriggers`].
///
/// Two operations in one pass:
/// 1. Drop every existing entry whose entity+relic combination
///    is no longer equipped (unequipped relic → no fires).
/// 2. Add an entry for every equipped relic's `Periodic` trigger
///    that isn't already scheduled.
///
/// Change detection on [`Loadout`] means this only runs when
/// equipment actually shifts — quiet in the common case.
fn sync_periodic_triggers(
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    changed: Query<(Entity, &Loadout), (With<NpcMarker>, Changed<Loadout>, Without<Dead>)>,
    dead_or_despawned: Query<Entity, Or<(With<Dead>, Without<NpcMarker>)>>,
) {
    if changed.is_empty() && periodic.entries.is_empty() {
        return;
    }
    let now = clock.0;
    let items = &data.0.items;

    // For every entity whose loadout changed this frame, build
    // a set of (relic_id, period_minutes, effect) tuples the
    // entity currently carries. Any existing PeriodicTriggers
    // entry for this entity that isn't in the set gets dropped;
    // any tuple not yet in the entries gets added.
    for (entity, loadout) in &changed {
        let mut carried: Vec<(Id<Item>, u32, TimedEffect)> = Vec::new();
        for relic_instance in &loadout.relics {
            let Some(def) = items.get(&relic_instance.def_id) else {
                continue;
            };
            let ItemData::Relic(relic) = &def.data else {
                continue;
            };
            for triggered in &relic.triggered {
                if let EffectTrigger::Periodic(duration) = triggered.trigger {
                    let period = duration.minutes();
                    if period == 0 {
                        warn!(
                            "relic `{}` has Periodic trigger with zero-minute \
                             duration — skipping",
                            relic_instance.def_id.as_str()
                        );
                        continue;
                    }
                    carried.push((relic_instance.def_id.clone(), period, triggered.effect));
                }
            }
        }

        // Drop stale entries for this entity.
        periodic.entries.retain(|e| {
            if e.entity != entity {
                return true;
            }
            carried
                .iter()
                .any(|(relic, period, _)| e.relic == *relic && e.period_minutes == *period)
        });

        // Add entries that don't exist yet.
        for (relic, period, effect) in carried {
            let already = periodic
                .entries
                .iter()
                .any(|e| e.entity == entity && e.relic == relic && e.period_minutes == period);
            if already {
                continue;
            }
            let mut next_fire = now;
            next_fire.advance_minutes(period);
            periodic.entries.push(PeriodicEntry {
                entity,
                relic,
                effect,
                period_minutes: period,
                next_fire,
            });
        }
    }

    // Also drop entries whose entity is dead or despawned.
    // Bevy change detection doesn't fire for despawns, so we
    // catch them by filtering the entries list.
    let dead_set: HashMap<Entity, ()> = dead_or_despawned
        .iter()
        .map(|e| (e, ()))
        .collect();
    periodic
        .entries
        .retain(|e| !dead_set.contains_key(&e.entity));
}

/// Fire every periodic entry whose `next_fire` has landed.
///
/// Applies the effect to the carrier and re-schedules the
/// entry for `next_fire + period_minutes`. If the carrier's
/// pool components don't match the entity (e.g. it's been
/// despawned since the last sync), the entry is dropped.
fn fire_periodic_triggers(
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    mut carriers: Query<
        (
            &mut ActiveEffects,
            &mut Hp,
            &mut StaminaPool,
            &mut HungerPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    // Collect which entries fire this frame so we can mutate
    // the resource after the carrier query releases. An entry
    // is "due" when `now >= entry.next_fire` — expressed as
    // `minutes_since > 0` (now is strictly after) OR
    // `now == next_fire` (now lands exactly on the minute).
    let mut fires: Vec<usize> = Vec::new();
    for (idx, entry) in periodic.entries.iter().enumerate() {
        let due = now.minutes_since(entry.next_fire) > 0 || now == entry.next_fire;
        if due {
            fires.push(idx);
        }
    }

    for idx in &fires {
        let entry = periodic.entries[*idx].clone();
        let Ok((mut active, mut hp, mut stamina, mut hunger, mut corruption)) =
            carriers.get_mut(entry.entity)
        else {
            continue;
        };
        apply_or_queue(
            entry.effect,
            ActiveEffectSource::PeriodicRelic(entry.relic),
            now,
            &mut active,
            &mut hp,
            &mut stamina,
            &mut hunger,
            &mut corruption,
        );
    }

    // Re-schedule or drop.
    for idx in fires.into_iter().rev() {
        let entry = &mut periodic.entries[idx];
        // Advance next_fire until it lands in the future.
        // Handles clock stutter where multiple periods may
        // have elapsed in one frame (fire one, skip the rest).
        while now.minutes_since(entry.next_fire) > 0 || now == entry.next_fire {
            entry.next_fire.advance_minutes(entry.period_minutes);
        }
    }
}

/// Advance every active effect by the number of minutes that
/// rolled over this frame, applying per-minute values and
/// dropping expired effects.
///
/// Instant effects never reach this system — they're applied
/// synchronously inside [`apply_or_queue`]. This tick only
/// walks entries that had a non-instant duration at creation.
fn tick_active_effects(
    clock: Res<GameClock>,
    mut carriers: Query<
        (
            &mut ActiveEffects,
            &mut Hp,
            &mut StaminaPool,
            &mut HungerPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    for (mut active, mut hp, mut stamina, mut hunger, mut corruption) in &mut carriers {
        // Walk indices so we can `swap_remove` expired entries
        // without invalidating iteration. Read the fields we
        // need up-front so the outer borrow of `active` is
        // released before `apply_pool_delta` runs — it doesn't
        // touch `active` but the compiler wants the shape to
        // be obvious.
        let mut i = 0;
        while i < active.effects.len() {
            let target = active.effects[i].effect.target;
            let value = active.effects[i].effect.value;
            let duration = active.effects[i].effect.duration.minutes();
            let last_tick_at = active.effects[i].last_tick_at;
            let started_at = active.effects[i].started_at;

            let elapsed_since_tick = now.minutes_since(last_tick_at);
            let total_elapsed = now.minutes_since(started_at);

            // Apply one per-minute tick for each whole minute
            // that rolled over since last_tick_at.
            if elapsed_since_tick > 0 {
                for _ in 0..elapsed_since_tick {
                    apply_pool_delta(
                        target,
                        value,
                        &mut hp,
                        &mut stamina,
                        &mut hunger,
                        &mut corruption,
                    );
                }
                active.effects[i].last_tick_at = now;
            }

            // Expiry check: drop if lifetime exceeded.
            if total_elapsed >= duration {
                active.effects.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }
}

/// Core dispatch primitive: apply an effect immediately (for
/// instant effects) or enqueue it into [`ActiveEffects`] (for
/// timed effects).
///
/// Takes every pool component mutably because the apply path
/// needs to touch whichever resource target the effect names.
/// The caller is the one holding the `&mut` borrows from its
/// own query, so this is a plain function, not a system or a
/// method.
#[allow(clippy::too_many_arguments)]
fn apply_or_queue(
    effect: TimedEffect,
    source: ActiveEffectSource,
    now: GameTime,
    active: &mut ActiveEffects,
    hp: &mut Hp,
    stamina: &mut StaminaPool,
    hunger: &mut HungerPool,
    corruption: &mut CorruptionPool,
) {
    if effect.duration.is_instant() {
        apply_pool_delta(effect.target, effect.value, hp, stamina, hunger, corruption);
        return;
    }
    active.effects.push(ActiveEffect {
        effect,
        source,
        started_at: now,
        last_tick_at: now,
    });
}

/// Apply a single `(target, value)` delta to the right pool.
///
/// Positive values restore (heal, feed, gain corruption), negative
/// values deplete. `Damage` is distinct from negative-`Health`
/// by authorial intent — using it reads as "this is damage",
/// using `Health` with a negative value reads as "this is a
/// drain or cost".
fn apply_pool_delta(
    target: ResourceTarget,
    value: f32,
    hp: &mut Hp,
    stamina: &mut StaminaPool,
    hunger: &mut HungerPool,
    corruption: &mut CorruptionPool,
) {
    match target {
        ResourceTarget::Health => apply_signed(hp, value),
        ResourceTarget::Damage => hp.deplete(value.abs() as u32),
        ResourceTarget::Stamina => apply_signed(stamina, value),
        ResourceTarget::Hunger => apply_signed(hunger, value),
        ResourceTarget::Corruption => apply_signed(corruption, value),
    }
}

/// Apply a signed `f32` delta to a pool: positive restores,
/// negative depletes, zero is a no-op.
fn apply_signed<K: cordon_core::primitive::PoolKind>(
    pool: &mut cordon_core::primitive::Pool<K>,
    value: f32,
) {
    if value > 0.0 {
        pool.restore(value as u32);
    } else if value < 0.0 {
        pool.deplete(value.abs() as u32);
    }
}
