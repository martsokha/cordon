//! World state and the (still mostly pre-ECS) world tick logic.
//!
//! This module is being progressively dissolved: each field on
//! [`state::World`] is migrating to its own Bevy resource or to a set
//! of entities. For now it still holds time, RNG, market, areas,
//! events, missions, and quests in one struct.
//!
//! Submodules:
//! - [`state`]   – the [`World`](state::World) struct itself
//! - [`market`]  – live market supply/demand state
//! - [`sectors`] – live area state
//! - [`day`]     – the day-rollover orchestrator
//! - [`events`]  – event scheduling and expiry
//! - [`factions`] – faction reactions to events
//! - [`generator`] – NPC and squad rolling (used by the spawn system)
//! - [`loadout`] – per-NPC loadout rolling

pub mod day;
pub mod events;
pub mod factions;
pub mod generator;
pub mod loadout;
pub mod market;
pub mod sectors;
pub mod state;
