#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// ECS components for NPCs and squads.
pub mod components;

/// Bevy plugin entry point.
pub mod plugin;

/// Top-level Bevy resources owned by cordon-sim.
pub mod resources;

/// NPC and squad spawning systems.
pub mod spawn;

/// World state, day cycle, event scheduling, faction dynamics, NPC
/// generation. Being progressively dissolved into ECS resources +
/// systems.
pub mod world;
