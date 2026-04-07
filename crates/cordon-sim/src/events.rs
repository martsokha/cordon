//! Sim → game events.
//!
//! cordon-sim emits these for cordon-bevy (or any other client) to
//! consume. Sim systems never reach into visuals or audio directly —
//! they write events here, and visual/audio systems subscribe via
//! `EventReader`. This is the only outgoing channel from the sim.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::item::Item;
use cordon_core::primitive::{Day, Id};

/// A weapon discharged from `from` toward `to`. The visual layer
/// renders a tracer; the audio layer plays a gunshot.
#[derive(Message, Debug, Clone, Copy)]
pub struct ShotFired {
    pub shooter: Entity,
    pub from: Vec2,
    pub to: Vec2,
}

/// An NPC's HP just hit zero. The death visual layer turns the dot
/// into an X marker; the AI cleanup pass removes the squad if every
/// member is dead.
#[derive(Message, Debug, Clone, Copy)]
pub struct NpcDied {
    pub entity: Entity,
    pub killer: Option<Entity>,
}

/// A dead NPC's loadout has been fully drained or its persistence
/// window has elapsed; the entity has been despawned this frame.
#[derive(Message, Debug, Clone, Copy)]
pub struct CorpseRemoved {
    pub entity: Entity,
}

/// A looter pulled an item from a corpse into their general pouch.
#[derive(Message, Debug, Clone)]
pub struct ItemLooted {
    pub looter: Entity,
    pub corpse: Entity,
    pub item: Id<Item>,
}

/// A fresh squad just entered the world.
#[derive(Message, Debug, Clone)]
pub struct SquadSpawned {
    pub entity: Entity,
    pub faction: Id<Faction>,
}

/// In-game day advanced.
#[derive(Message, Debug, Clone, Copy)]
pub struct DayRolled {
    pub new_day: Day,
}
