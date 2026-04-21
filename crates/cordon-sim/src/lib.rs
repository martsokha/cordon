#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
// Bevy systems naturally have many resource params and complex Query
// types — these lints fire on idiomatic Bevy code, so they're allowed
// crate-wide rather than per-system.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

/// Per-entity behavior subplugins: movement, vision, combat, death,
/// loot. Each subplugin follows the
/// `{mod, components, systems, events?, constants?}` file convention.
pub mod behavior;

/// Bunker-driven sim systems: upgrade shop, pill dose tracking.
/// Each submodule hosts the sim-side data + handlers for one
/// bunker interaction.
pub mod bunker;

/// Day rollover detection and per-day systems.
pub mod day;

/// Per-entity ECS components that aren't owned by a specific
/// behavior subplugin — NPC attributes, relic markers.
pub mod entity;

/// Bevy plugin entry point.
pub mod plugin;

/// Runtime quest state, condition evaluation, consequence
/// application, and trigger dispatch.
pub mod quest;

/// Top-level Bevy resources owned by cordon-sim.
pub mod resources;

/// NPC and squad spawning systems.
pub mod spawn;
