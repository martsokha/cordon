//! Effect dispatcher.
//!
//! Drives timed and triggered effects at runtime. Connects three
//! inputs to one per-entity state (`ActiveEffects`) and one global
//! scheduler ([`PeriodicTriggers`]):
//!
//! - `NpcPoolChanged` messages from combat and from the dispatcher's
//!   own pool writes fire every reactive trigger: `OnHit` on any HP
//!   decrease, plus edge-triggered `OnLow*` / `OnHigh*` variants
//!   when a pool crosses its threshold.
//! - Loadout changes (add/remove relic) register or prune periodic
//!   entries.
//! - The game clock's minute rollover ticks every active effect and
//!   fires eligible periodic entries.
//!
//! Runs in [`SimSet::Effects`] between combat and death so an
//! `OnLowHealth` heal can save a carrier in the same frame combat
//! depleted them.
//!
//! # File layout
//!
//! - [`apply`]      — `apply_or_queue` / `apply_pool_delta`: the
//!   shared primitives every effect path funnels through.
//! - [`reactive`]   — `dispatch_pool_triggers`: react to pool
//!   changes and fire `OnHit` / `OnLow*` / `OnHigh*` triggers.
//! - [`scheduler`]  — `PeriodicTriggers` resource plus the
//!   `sync_periodic_triggers` / `fire_periodic_triggers` pair
//!   driving the periodic-fire schedule.
//! - [`tick`]       — `tick_active_effects`: advance timed entries
//!   minute-by-minute and drop expired ones.
//! - [`consume`]    — NPC-autonomous consumable use based on need
//!   thresholds.
//! - [`corruption`] — area-corruption tick for NPCs standing in
//!   anomaly zones.
//! - [`throwable`]  — grenade throws and on-impact resolution.
//!
//! # What each `TimedEffect::target` does here
//!
//! | Target | Handler |
//! |---|---|
//! | `Health` | `hp.restore` for positive, `hp.deplete` for negative. Positive clamps at max. |
//! | `Damage` | `hp.deplete(value as u32)`. Negative values are rejected with a warning — use `Health` to heal. |
//! | `Stamina` | `stamina.restore` / `deplete`. |
//! | `Corruption` | `corruption.restore` for positive (gain corruption), `deplete` for negative (scrubbing). |
//!
//! `Bleeding`, `Poison`, and `Smoke` were deleted from
//! `ResourceTarget` before this commit — status flags and area
//! regions need different data shapes and will come back when their
//! respective subsystems land.

mod apply;
mod consume;
mod corruption;
mod reactive;
mod scheduler;
mod throwable;
mod tick;

use bevy::prelude::*;

pub use scheduler::PeriodicTriggers;
pub use throwable::ThrowableImpact;

use crate::plugin::SimSet;

/// Plugin registering the effect dispatcher.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PeriodicTriggers>();
        app.add_message::<ThrowableImpact>();
        // Source systems run first so the events they emit are
        // visible to dispatch_pool_triggers in the same frame.
        // Periodic sync runs interleaved with the others — it only
        // walks Changed<Loadout> entries so it's quiet in the
        // steady state.
        app.add_systems(
            Update,
            (
                throwable::npc_throw_grenades,
                throwable::process_throwable_impacts,
                consume::npc_auto_consume,
                corruption::area_corruption_tick,
                scheduler::sync_periodic_triggers,
                reactive::dispatch_pool_triggers,
                scheduler::fire_periodic_triggers,
                tick::tick_active_effects,
            )
                .chain()
                .in_set(SimSet::Effects),
        );
    }
}
