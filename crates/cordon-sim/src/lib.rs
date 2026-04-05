#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Day cycle, event scheduling, mission resolution, NPC spawning, faction dynamics.
pub mod simulation;

/// Mutable world state: market, sectors, and the top-level world struct.
pub mod state;
