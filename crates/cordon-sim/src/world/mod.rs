//! Helper modules used by the spawn pipeline.
//!
//! What used to be a "simulation orchestrator" operating on a giant
//! `World` struct is now empty: each former concern is its own Bevy
//! resource (see [`crate::resources`]). The day-cycle helpers
//! (events, factions, missions) will come back as event-driven
//! systems when those features are wired up.

pub mod events;
pub mod factions;
pub mod generator;
pub mod loadout;
pub mod sectors;
