//! Death-related messages (events) consumed by the visual layer and
//! the quest/AI systems.

use bevy::prelude::*;

/// An NPC's HP just hit zero. The death visual layer turns the
/// dot into an X marker; the AI cleanup pass removes the squad
/// if every member is dead.
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
