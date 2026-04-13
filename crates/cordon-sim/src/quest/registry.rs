//! Template registry: tracks which NPC templates are currently
//! alive, which have been permanently killed, and which entities
//! back them. Quest conditions (`NpcAlive`, `NpcDead`) and
//! consequence messages (`SpawnNpcRequest`, `GiveNpcXpRequest`)
//! go through this resource.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::primitive::Id;

/// Maps template IDs to their live Bevy entities and tracks
/// permanent deaths so `SpawnNpc` can refuse to resurrect a
/// non-respawnable character.
#[derive(Resource, Default)]
pub struct TemplateRegistry {
    alive: HashMap<Id<NpcTemplate>, Entity>,
    permanently_dead: HashSet<Id<NpcTemplate>>,
}

impl TemplateRegistry {
    pub fn is_alive(&self, id: &Id<NpcTemplate>) -> bool {
        self.alive.contains_key(id)
    }

    pub fn is_permanently_dead(&self, id: &Id<NpcTemplate>) -> bool {
        self.permanently_dead.contains(id)
    }

    pub fn entity(&self, id: &Id<NpcTemplate>) -> Option<Entity> {
        self.alive.get(id).copied()
    }

    pub fn register(&mut self, id: Id<NpcTemplate>, entity: Entity) {
        self.alive.insert(id, entity);
    }

    pub fn mark_dead(&mut self, id: &Id<NpcTemplate>, permanent: bool) {
        self.alive.remove(id);
        if permanent {
            self.permanently_dead.insert(id.clone());
        }
    }
}
