//! Active-effect tick: advance every queued [`TimedEffect`] by the
//! minutes that rolled over this frame, apply per-minute values,
//! drop expired entries.
//!
//! Instant effects never reach this system — they're applied
//! synchronously inside [`super::apply::apply_or_queue`]. This tick
//! only walks entries that had a non-instant duration at creation.

use bevy::prelude::*;
use cordon_core::primitive::{Corruption, Health, Pool, Stamina};

use super::apply::apply_pool_delta;
use crate::behavior::combat::NpcPoolChanged;
use crate::behavior::death::Dead;
use crate::entity::npc::ActiveEffects;
use crate::resources::GameClock;

/// Advance every active effect by the number of minutes that
/// rolled over this frame, applying per-minute values and
/// dropping expired effects.
pub(super) fn tick_active_effects(
    clock: Res<GameClock>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut carriers: Query<
        (
            Entity,
            &mut ActiveEffects,
            &mut Pool<Health>,
            &mut Pool<Stamina>,
            &mut Pool<Corruption>,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    for (entity, mut active, mut hp, mut stamina, mut corruption) in &mut carriers {
        // Walk indices so we can `swap_remove` expired entries
        // without invalidating iteration. Read the fields we need
        // up-front so the outer borrow of `active` is released
        // before `apply_pool_delta` runs.
        let mut i = 0;
        while i < active.effects.len() {
            let target = active.effects[i].effect.target;
            let value = active.effects[i].effect.value;
            let duration = active.effects[i].effect.duration.minutes();
            let last_tick_at = active.effects[i].last_tick_at;
            let started_at = active.effects[i].started_at;

            let elapsed_since_tick = now.minutes_since(last_tick_at);
            let total_elapsed = now.minutes_since(started_at);

            // Tick count is the number of whole minutes that rolled
            // over since last_tick_at, capped at the effect's
            // remaining lifetime. Without the cap, a frame stutter
            // that advances the clock past the effect's expiry would
            // overrun the tick count by (stutter - remaining) extra
            // applies before the expiry check below drops the entry.
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
