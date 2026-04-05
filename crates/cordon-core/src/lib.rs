#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Bunker state, upgrades, and storage management.
pub mod bunker;

/// Items, pricing, and mission types.
pub mod economy;

/// NPCs, factions, and the player.
pub mod entity;

/// Identifiers for data-driven and runtime objects.
pub mod object;

/// Time, sectors, and zone events.
pub mod world;
