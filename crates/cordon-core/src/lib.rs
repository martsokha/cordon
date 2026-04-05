#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Bunker state, upgrades, and storage management.
pub mod bunker;

/// NPCs, factions, and the player.
pub mod entity;

/// Item definitions, effects, calibers, and instances.
pub mod item;

/// Primitive value types: identifiers, condition, duration.
pub mod primitive;

/// Time, sectors, zone events, pricing, and missions.
pub mod world;
