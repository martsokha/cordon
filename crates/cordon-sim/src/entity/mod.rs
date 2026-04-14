//! ECS components for per-entity state that isn't specific to any
//! one behavior subplugin: NPCs and relics live here. Per-squad
//! components live under [`crate::behavior::squad`] (split across
//! `identity`, `intent`, `formation` by ownership).
//!
//! `Uid<Npc>` and `Uid<Squad>` still exist as stable identifiers
//! for save-game and quest references, but runtime systems use
//! `Entity` to look things up because it's an O(1) array index in
//! Bevy.

pub mod npc;
pub mod relic;
