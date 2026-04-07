//! ECS components for NPCs and squads.
//!
//! `Uid<Npc>` and `Uid<Squad>` still exist as stable identifiers for
//! save-game and quest references, but runtime systems use `Entity`
//! to look things up because it's an O(1) array index in Bevy.

pub mod npc;
pub mod squad;

pub use npc::*;
pub use squad::*;
