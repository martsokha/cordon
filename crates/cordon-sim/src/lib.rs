#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// ECS components for NPCs and squads.
pub mod components;

/// Bevy plugin entry point.
pub mod plugin;

/// Day cycle, event scheduling, mission resolution, NPC spawning, faction dynamics.
pub mod simulation;

/// NPC and squad spawning systems.
pub mod spawn;

/// Mutable world state: market, sectors, and the top-level world struct.
pub mod state;
