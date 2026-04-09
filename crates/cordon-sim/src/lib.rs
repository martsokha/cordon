#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

/// Per-NPC behavior state and the per-NPC movement system.
pub mod behavior;

/// Combat resolution: weapon firing, damage, hostility checks.
pub mod combat;

/// ECS components for NPCs and squads.
pub mod components;

/// Day rollover detection and per-day systems.
pub mod day;

/// Death and corpse lifecycle.
pub mod death;

/// Effect dispatcher: ActiveEffects tick, OnHit/OnHpLow/Periodic
/// relic-trigger wiring, and the TimedEffect → pool handler.
pub mod effects;

/// Looting: alive NPCs pull items from nearby corpses.
pub mod loot;

/// Bevy plugin entry point.
pub mod plugin;

/// Runtime quest state, condition evaluation, consequence
/// application, and trigger dispatch.
pub mod quest;

/// Top-level Bevy resources owned by cordon-sim.
pub mod resources;

/// NPC and squad spawning systems.
pub mod spawn;

/// Squad behavior: engagement, formation, goals, lifecycle, and the
/// player command boundary.
pub mod squad;

/// Gameplay tuning knobs — distances, timings, thresholds,
/// probabilities. One place to tune the sim from.
pub mod tuning;
