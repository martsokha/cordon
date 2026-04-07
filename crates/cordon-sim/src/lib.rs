#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

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

/// Sim → game events emitted on the boundary.
pub mod events;

/// Looting: alive NPCs pull items from nearby corpses.
pub mod loot;

/// Bevy plugin entry point.
pub mod plugin;

/// Top-level Bevy resources owned by cordon-sim.
pub mod resources;

/// NPC and squad spawning systems.
pub mod spawn;

/// Squad AI: engagement, formation, goal transitions, lifecycle.
pub mod squad_ai;

/// World state, day cycle, event scheduling, faction dynamics, NPC
/// generation. Being progressively dissolved into ECS resources +
/// systems.
pub mod world;
