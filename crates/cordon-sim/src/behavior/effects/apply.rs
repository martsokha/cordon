//! Core application primitives: turn one `(target, value)` delta
//! into the right pool mutation and emit an [`NpcPoolChanged`]
//! describing what moved.
//!
//! These are plain functions, not systems — they take `&mut`
//! borrows from the caller's own query. Every effect-tick path
//! funnels through [`apply_or_queue`] so instant vs. timed
//! behaviour lives in one place.

use bevy::prelude::*;
use cordon_core::item::{ResourceTarget, TimedEffect};
use cordon_core::primitive::{Corruption, GameTime, Health, Pool, PoolKind, Stamina};

use crate::behavior::combat::NpcPoolChanged;
use crate::entity::npc::{ActiveEffect, ActiveEffects};

/// Dispatch an effect: apply it immediately (for instant effects)
/// or enqueue it into [`ActiveEffects`] (for timed effects).
///
/// Takes every pool component mutably because the apply path needs
/// to touch whichever resource target the effect names. The caller
/// holds the `&mut` borrows from its own query, so this is a plain
/// function, not a system or method.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_or_queue(
    entity: Entity,
    effect: TimedEffect,
    now: GameTime,
    active: &mut ActiveEffects,
    hp: &mut Pool<Health>,
    stamina: &mut Pool<Stamina>,
    corruption: &mut Pool<Corruption>,
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
/// values deplete. `Damage` is distinct from negative-`Health` by
/// authorial intent — using it reads as "this is damage", using
/// `Health` with a negative value reads as "this is a drain or
/// cost". Both emit as a `Health` pool change because the
/// downstream bus only cares about which pool moved, not about
/// authorial flavour. A negative value on `Damage` is treated as
/// zero (with a warning) because the sign isn't meaningful there.
pub(crate) fn apply_pool_delta(
    entity: Entity,
    target: ResourceTarget,
    value: f32,
    hp: &mut Pool<Health>,
    stamina: &mut Pool<Stamina>,
    corruption: &mut Pool<Corruption>,
    pool_changed: &mut MessageWriter<NpcPoolChanged>,
) {
    match target {
        ResourceTarget::Health => {
            emit_signed(entity, ResourceTarget::Health, hp, value, pool_changed);
        }
        ResourceTarget::Damage => {
            if value < 0.0 {
                warn!(
                    "effect: `Damage` target received negative value {value} — \
                     treating as 0. Use `Health` with a positive value to heal."
                );
                return;
            }
            let prev = hp.current();
            hp.deplete(value as u32);
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
            emit_signed(
                entity,
                ResourceTarget::Stamina,
                stamina,
                value,
                pool_changed,
            );
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
fn emit_signed<K: PoolKind>(
    entity: Entity,
    kind: ResourceTarget,
    pool: &mut Pool<K>,
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
fn apply_signed<K: PoolKind>(pool: &mut Pool<K>, value: f32) {
    if value > 0.0 {
        pool.restore(value as u32);
    } else if value < 0.0 {
        pool.deplete(value.abs() as u32);
    }
}
