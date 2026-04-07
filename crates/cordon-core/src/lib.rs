#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

/// NPCs, factions, the player, bunker, and perks.
pub mod entity;

/// Item definitions, effects, calibers, and instances.
pub mod item;

/// Primitive value types: identifiers, condition, duration.
pub mod primitive;

/// Time, sectors, zone events, pricing, and missions.
pub mod world;
