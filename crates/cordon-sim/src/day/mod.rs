//! Day rollover detection and the per-day systems.
//!
//! [`systems::detect_day_rollover`] watches [`GameClock`] each frame
//! and writes a [`DayRolled`] message whenever the day number
//! advances. Per-day work — daily world-event rolls, faction
//! reactions, event expiry — runs as separate systems gated on the
//! message.
//!
//! # File layout
//!
//! - [`events`] — ECS `Message` types emitted by this plugin
//!   (currently just [`DayRolled`]).
//! - [`systems`] — the `detect_day_rollover` / `roll_today_events` /
//!   `expire_old_events` systems.
//! - [`world_events`] — pure functions for rolling and expiring
//!   in-world `ActiveEvent`s (faction_war, coup, radiation_storm).
//!   Name is deliberately distinct from `events` to separate
//!   world-state events from ECS messages.

pub mod events;
pub mod payroll;
pub mod radio;
pub mod systems;
pub mod world_events;

use bevy::prelude::*;
pub use events::DayRolled;

use crate::plugin::SimSet;

pub struct DayCyclePlugin;

impl Plugin for DayCyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DayRolled>();
        app.add_message::<radio::RadioBroadcast>();
        app.add_message::<radio::BroadcastHeard>();
        app.init_resource::<payroll::LastDailyExpenses>();
        app.init_resource::<radio::DeliveredBroadcasts>();
        app.add_systems(
            Update,
            (
                systems::detect_day_rollover,
                systems::roll_today_events.run_if(on_message::<DayRolled>),
                systems::expire_old_events.run_if(on_message::<DayRolled>),
                systems::expire_old_intel.run_if(on_message::<DayRolled>),
                radio::prune_delivered_broadcasts.run_if(on_message::<DayRolled>),
                payroll::process_daily_expenses.run_if(on_message::<DayRolled>),
            )
                .chain()
                .in_set(SimSet::Cleanup),
        );
        // Radio delivery runs every frame (not just on day rollover)
        // so broadcasts fire as soon as the delay elapses.
        app.add_systems(
            Update,
            (
                radio::process_broadcast_heard,
                radio::deliver_radio_broadcasts,
            )
                .chain()
                .in_set(SimSet::Cleanup),
        );
    }
}
