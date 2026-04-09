//! Effect dispatcher.
//!
//! Drives [`TimedEffect`] and [`TriggeredEffect`] at runtime.
//! Connects three inputs to one per-entity state
//! ([`ActiveEffects`]) and one global scheduler
//! ([`PeriodicTriggers`]):
//!
//! - [`NpcPoolChanged`] messages from combat and from the
//!   dispatcher's own pool writes fire every reactive trigger:
//!   `OnHit` on any HP decrease, plus edge-triggered
//!   `OnLow*` / `OnHigh*` variants when a pool crosses its
//!   threshold.
//! - Loadout changes (add/remove relic) register or prune
//!   periodic entries.
//! - The game clock's minute rollover ticks every active
//!   effect and fires eligible periodic entries.
//!
//! Runs in [`SimSet::Effects`] between combat and death so an
//! `OnLowHealth` heal can save a carrier in the same frame combat
//! depleted them.
//!
//! # What each `TimedEffect::target` does here
//!
//! | Target | Handler |
//! |---|---|
//! | `Health` | `hp.restore` for positive, `hp.deplete` for negative. Positive clamps at max. |
//! | `Damage` | `hp.deplete(value as u32)`. Distinct from `Health` by authorial intent. |
//! | `Stamina` | `stamina.restore` / `deplete`. |
//! | `Corruption` | `corruption.restore` for positive (gain corruption), `deplete` for negative (scrubbing). |
//!
//! `Bleeding`, `Poison`, and `Smoke` were deleted from
//! [`ResourceTarget`](cordon_core::item::ResourceTarget)
//! before this commit — status flags and area regions need
//! different data shapes and will come back when their
//! respective subsystems land.

use bevy::prelude::*;
use cordon_core::item::{
    CORRUPTION_HIGH_THRESHOLD, CORRUPTION_LOW_THRESHOLD, EffectTrigger, HP_HIGH_THRESHOLD,
    HP_LOW_THRESHOLD, Item, ItemData, Loadout, PERIODIC_INTERVAL_MINUTES, ResourceTarget,
    STAMINA_HIGH_THRESHOLD, STAMINA_LOW_THRESHOLD, TimedEffect,
};
use cordon_core::primitive::{GameTime, Id};
use cordon_data::gamedata::GameDataResource;

use crate::behavior::Dead;
use crate::combat::NpcPoolChanged;
use crate::components::{
    ActiveEffect, ActiveEffects, CorruptionPool, HealthPool, NpcMarker, StaminaPool,
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
                dispatch_pool_triggers,
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

/// One scheduled periodic trigger. Every periodic relic fires on
/// the same fixed cadence ([`PERIODIC_INTERVAL_MINUTES`]), so the
/// entry doesn't store its own period.
#[derive(Debug, Clone)]
struct PeriodicEntry {
    /// Which entity carries the relic.
    entity: Entity,
    /// Which relic produced this trigger. Used to prune on
    /// loadout change.
    relic: Id<Item>,
    /// The effect to apply on each fire.
    effect: TimedEffect,
    /// When the next fire is scheduled. Compared against
    /// `GameClock::now` each tick.
    next_fire: GameTime,
}

/// Read [`NpcPoolChanged`] messages and fire any matching
/// reactive relic effects on the affected entity.
///
/// This is the single entry point for every non-periodic trigger:
/// - [`EffectTrigger::OnHit`] fires when a `Health` event carries
///   a negative delta (damage taken).
/// - [`EffectTrigger::OnLowHealth`] / [`OnHighHealth`] fire
///   edge-triggered when `prev` crossed the threshold but
///   `current` didn't.
/// - The stamina and corruption variants work the same way on
///   their respective pools.
///
/// Runs before [`tick_active_effects`] so instant-effect heals
/// land the same frame the damage was applied — which is why an
/// `OnLowHealth` heal can save a carrier that just hit zero HP.
/// The query filters `Without<Dead>` because dead entities don't
/// get new triggers (the frame's already resolved).
fn dispatch_pool_triggers(
    mut messages: ParamSet<(
        MessageReader<NpcPoolChanged>,
        MessageWriter<NpcPoolChanged>,
    )>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut targets: Query<
        (
            &Loadout,
            &mut ActiveEffects,
            &mut HealthPool,
            &mut StaminaPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    let items = &data.0.items;

    // Drain into a Vec so we can re-emit further events later
    // (instant-apply side effects emit their own NpcPoolChanged)
    // without aliasing the reader — and so that the writer borrow
    // below doesn't overlap the reader borrow.
    let events: Vec<NpcPoolChanged> = messages.p0().read().copied().collect();
    let mut pool_changed = messages.p1();

    for event in events {
        let Ok((loadout, mut active, mut hp, mut stamina, mut corruption)) =
            targets.get_mut(event.entity)
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
                if !trigger_matches(&triggered.trigger, &event) {
                    continue;
                }
                apply_or_queue(
                    event.entity,
                    triggered.effect,
                    now,
                    &mut active,
                    &mut hp,
                    &mut stamina,
                    &mut corruption,
                    &mut pool_changed,
                );
            }
        }
    }
}

/// Decide whether `trigger` should fire for `event`. Threshold
/// variants are edge-triggered: they only fire on the frame the
/// pool crosses the threshold, never while the pool sits on the
/// wrong side.
fn trigger_matches(trigger: &EffectTrigger, event: &NpcPoolChanged) -> bool {
    let max = event.max as f32;
    if max <= 0.0 {
        return false;
    }
    match trigger {
        // OnHit: any HP decrease.
        EffectTrigger::OnHit => {
            event.pool == ResourceTarget::Health && event.current < event.prev
        }
        EffectTrigger::OnLowHealth => {
            event.pool == ResourceTarget::Health && crossed_low(event, max, HP_LOW_THRESHOLD)
        }
        EffectTrigger::OnHighHealth => {
            event.pool == ResourceTarget::Health && crossed_high(event, max, HP_HIGH_THRESHOLD)
        }
        EffectTrigger::OnLowStamina => {
            event.pool == ResourceTarget::Stamina
                && crossed_low(event, max, STAMINA_LOW_THRESHOLD)
        }
        EffectTrigger::OnHighStamina => {
            event.pool == ResourceTarget::Stamina
                && crossed_high(event, max, STAMINA_HIGH_THRESHOLD)
        }
        EffectTrigger::OnLowCorruption => {
            event.pool == ResourceTarget::Corruption
                && crossed_low(event, max, CORRUPTION_LOW_THRESHOLD)
        }
        EffectTrigger::OnHighCorruption => {
            event.pool == ResourceTarget::Corruption
                && crossed_high(event, max, CORRUPTION_HIGH_THRESHOLD)
        }
        // Periodic is scheduled separately.
        EffectTrigger::Periodic => false,
    }
}

/// True if `event` crossed from above-or-at `max * ratio` to
/// strictly below it this frame.
fn crossed_low(event: &NpcPoolChanged, max: f32, ratio: f32) -> bool {
    let threshold = max * ratio;
    (event.prev as f32) > threshold && (event.current as f32) <= threshold
}

/// True if `event` crossed from below-or-at `max * ratio` to
/// strictly above it this frame.
fn crossed_high(event: &NpcPoolChanged, max: f32, ratio: f32) -> bool {
    let threshold = max * ratio;
    (event.prev as f32) < threshold && (event.current as f32) >= threshold
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
///
/// Dead-entity pruning uses `RemovedComponents<NpcMarker>` as
/// the signal rather than scanning every dead entity each
/// frame: when `NpcMarker` comes off an entity (because it
/// despawned, which takes the whole bundle with it), the
/// removed-components reader yields exactly those entities
/// this frame, and we drop their periodic entries. `Dead` is
/// a marker but despawn is the lifetime event we actually
/// care about, and removing the underlying NpcMarker is what
/// happens on despawn.
fn sync_periodic_triggers(
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    changed: Query<(Entity, &Loadout), (With<NpcMarker>, Changed<Loadout>, Without<Dead>)>,
    mut removed_npcs: RemovedComponents<NpcMarker>,
) {
    // Drop any entries whose carrier despawned this frame.
    // Cheap: the reader yields only removed entities, not the
    // whole alive-NPC set.
    let removed: Vec<Entity> = removed_npcs.read().collect();
    if !removed.is_empty() {
        periodic.entries.retain(|e| !removed.contains(&e.entity));
    }

    if changed.is_empty() {
        return;
    }
    let now = clock.0;
    let items = &data.0.items;

    // For every entity whose loadout changed this frame, build
    // the set of `(relic_id, effect)` pairs the entity currently
    // carries for periodic triggers. Any existing entry for this
    // entity that isn't in the set gets dropped; any pair not yet
    // in the entries gets added.
    for (entity, loadout) in &changed {
        let mut carried: Vec<(Id<Item>, TimedEffect)> = Vec::new();
        for relic_instance in &loadout.relics {
            let Some(def) = items.get(&relic_instance.def_id) else {
                continue;
            };
            let ItemData::Relic(relic) = &def.data else {
                continue;
            };
            for triggered in &relic.triggered {
                if triggered.trigger == EffectTrigger::Periodic {
                    carried.push((relic_instance.def_id.clone(), triggered.effect));
                }
            }
        }

        // Drop stale entries for this entity.
        periodic.entries.retain(|e| {
            if e.entity != entity {
                return true;
            }
            carried.iter().any(|(relic, _)| e.relic == *relic)
        });

        // Add entries that don't exist yet.
        for (relic, effect) in carried {
            let already = periodic
                .entries
                .iter()
                .any(|e| e.entity == entity && e.relic == relic);
            if already {
                continue;
            }
            let mut next_fire = now;
            next_fire.advance_minutes(PERIODIC_INTERVAL_MINUTES);
            periodic.entries.push(PeriodicEntry {
                entity,
                relic,
                effect,
                next_fire,
            });
        }
    }
}

/// Fire every periodic entry whose `next_fire` has landed.
///
/// Single pass: apply the effect, then advance `next_fire` to
/// the first minute strictly after `now`. The advance loop
/// handles clock stutter where multiple periods elapsed in one
/// frame by skipping the backlog (fire once, not N times).
fn fire_periodic_triggers(
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut carriers: Query<
        (
            &mut ActiveEffects,
            &mut HealthPool,
            &mut StaminaPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    for entry in periodic.entries.iter_mut() {
        let due = now.minutes_since(entry.next_fire) > 0 || now == entry.next_fire;
        if !due {
            continue;
        }
        if let Ok((mut active, mut hp, mut stamina, mut corruption)) =
            carriers.get_mut(entry.entity)
        {
            apply_or_queue(
                entry.entity,
                entry.effect,
                now,
                &mut active,
                &mut hp,
                &mut stamina,
                &mut corruption,
                &mut pool_changed,
            );
        }
        // Advance next_fire until it lands strictly in the
        // future. Dead-carrier entries are left in place and
        // cleaned up by `sync_periodic_triggers` when
        // `RemovedComponents<NpcMarker>` fires on despawn.
        while now.minutes_since(entry.next_fire) > 0 || now == entry.next_fire {
            entry.next_fire.advance_minutes(PERIODIC_INTERVAL_MINUTES);
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
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut carriers: Query<
        (
            Entity,
            &mut ActiveEffects,
            &mut HealthPool,
            &mut StaminaPool,
            &mut CorruptionPool,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    for (entity, mut active, mut hp, mut stamina, mut corruption) in &mut carriers {
        // Walk indices so we can `swap_remove` expired entries
        // without invalidating iteration. Read the fields we
        // need up-front so the outer borrow of `active` is
        // released before `apply_pool_delta` runs.
        let mut i = 0;
        while i < active.effects.len() {
            let target = active.effects[i].effect.target;
            let value = active.effects[i].effect.value;
            let duration = active.effects[i].effect.duration.minutes();
            let last_tick_at = active.effects[i].last_tick_at;
            let started_at = active.effects[i].started_at;

            let elapsed_since_tick = now.minutes_since(last_tick_at);
            let total_elapsed = now.minutes_since(started_at);

            // Tick count is the number of whole minutes that
            // rolled over since last_tick_at, capped at the
            // effect's remaining lifetime. Without the cap, a
            // frame stutter that advances the clock past the
            // effect's expiry would overrun the tick count by
            // (stutter - remaining) extra applies before the
            // expiry check below drops the entry.
            if elapsed_since_tick > 0 {
                let remaining = duration.saturating_sub(total_elapsed - elapsed_since_tick);
                let ticks = elapsed_since_tick.min(remaining);
                for _ in 0..ticks {
                    apply_pool_delta(
                        entity,
                        target,
                        value,
                        &mut hp,
                        &mut stamina,
                        &mut corruption,
                        &mut pool_changed,
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
    entity: Entity,
    effect: TimedEffect,
    now: GameTime,
    active: &mut ActiveEffects,
    hp: &mut HealthPool,
    stamina: &mut StaminaPool,
    corruption: &mut CorruptionPool,
    pool_changed: &mut MessageWriter<NpcPoolChanged>,
) {
    if effect.duration.is_instant() {
        apply_pool_delta(
            entity,
            effect.target,
            effect.value,
            hp,
            stamina,
            corruption,
            pool_changed,
        );
        return;
    }
    active.effects.push(ActiveEffect {
        effect,
        started_at: now,
        last_tick_at: now,
    });
}

/// Apply a single `(target, value)` delta to the right pool and
/// emit a [`NpcPoolChanged`] message describing the change.
///
/// Positive values restore (heal, gain corruption), negative
/// values deplete. `Damage` is distinct from negative-`Health`
/// by authorial intent — using it reads as "this is damage",
/// using `Health` with a negative value reads as "this is a
/// drain or cost" — but both emit as a `Health` pool change
/// because the downstream bus only cares about which pool moved,
/// not about authorial flavour.
fn apply_pool_delta(
    entity: Entity,
    target: ResourceTarget,
    value: f32,
    hp: &mut HealthPool,
    stamina: &mut StaminaPool,
    corruption: &mut CorruptionPool,
    pool_changed: &mut MessageWriter<NpcPoolChanged>,
) {
    match target {
        ResourceTarget::Health => {
            emit_signed(entity, ResourceTarget::Health, hp, value, pool_changed);
        }
        ResourceTarget::Damage => {
            let prev = hp.current();
            hp.deplete(value.abs() as u32);
            if hp.current() != prev {
                pool_changed.write(NpcPoolChanged {
                    entity,
                    pool: ResourceTarget::Health,
                    prev,
                    current: hp.current(),
                    max: hp.max(),
                });
            }
        }
        ResourceTarget::Stamina => {
            emit_signed(entity, ResourceTarget::Stamina, stamina, value, pool_changed);
        }
        ResourceTarget::Corruption => {
            emit_signed(
                entity,
                ResourceTarget::Corruption,
                corruption,
                value,
                pool_changed,
            );
        }
    }
}

/// Apply a signed value to `pool` and emit a [`NpcPoolChanged`]
/// if anything actually moved.
fn emit_signed<K: cordon_core::primitive::PoolKind>(
    entity: Entity,
    kind: ResourceTarget,
    pool: &mut cordon_core::primitive::Pool<K>,
    value: f32,
    pool_changed: &mut MessageWriter<NpcPoolChanged>,
) {
    let prev = pool.current();
    apply_signed(pool, value);
    let current = pool.current();
    if current != prev {
        pool_changed.write(NpcPoolChanged {
            entity,
            pool: kind,
            prev,
            current,
            max: pool.max(),
        });
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
