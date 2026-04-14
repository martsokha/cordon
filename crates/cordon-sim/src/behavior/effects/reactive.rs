//! Reactive relic triggers: react to [`NpcPoolChanged`] messages
//! and fire any matching `OnHit` / `OnLow*` / `OnHigh*` relic
//! effect on the affected entity.
//!
//! This is the single entry point for every non-periodic trigger
//! class. Threshold variants are edge-triggered — they fire only
//! on the frame the pool crosses the threshold, never while it
//! sits on the wrong side.

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use cordon_core::item::{
    CORRUPTION_HIGH_THRESHOLD, CORRUPTION_LOW_THRESHOLD, EffectTrigger, HP_HIGH_THRESHOLD,
    HP_LOW_THRESHOLD, ItemData, Loadout, ResourceTarget, STAMINA_HIGH_THRESHOLD,
    STAMINA_LOW_THRESHOLD,
};
use cordon_core::primitive::{Corruption, Health, Pool, Stamina};
use cordon_data::gamedata::GameDataResource;

use super::apply::apply_or_queue;
use crate::behavior::combat::NpcPoolChanged;
use crate::behavior::death::Dead;
use crate::entity::npc::ActiveEffects;
use crate::resources::GameClock;

/// Read [`NpcPoolChanged`] messages and fire any matching reactive
/// relic effects on the affected entity.
///
/// Runs before [`super::tick::tick_active_effects`] so instant
/// heals land the same frame the damage was applied — which is why
/// an `OnLowHealth` heal can save a carrier that just hit zero HP.
/// The query filters `Without<Dead>` because dead entities don't
/// get new triggers (the frame's already resolved).
pub(super) fn dispatch_pool_triggers(
    mut messages: ParamSet<(MessageReader<NpcPoolChanged>, MessageWriter<NpcPoolChanged>)>,
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut targets: Query<
        (
            &Loadout,
            &mut ActiveEffects,
            &mut Pool<Health>,
            &mut Pool<Stamina>,
            &mut Pool<Corruption>,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    let items = &data.0.items;

    // Drain into a Vec so we can re-emit further events later
    // (instant-apply side effects emit their own NpcPoolChanged)
    // without aliasing the reader — and so the writer borrow below
    // doesn't overlap the reader borrow.
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
        EffectTrigger::OnHit => event.pool == ResourceTarget::Health && event.current < event.prev,
        EffectTrigger::OnLowHealth => {
            event.pool == ResourceTarget::Health && crossed_low(event, max, HP_LOW_THRESHOLD)
        }
        EffectTrigger::OnHighHealth => {
            event.pool == ResourceTarget::Health && crossed_high(event, max, HP_HIGH_THRESHOLD)
        }
        EffectTrigger::OnLowStamina => {
            event.pool == ResourceTarget::Stamina && crossed_low(event, max, STAMINA_LOW_THRESHOLD)
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
        // Periodic is scheduled separately (see [`super::scheduler`]).
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
