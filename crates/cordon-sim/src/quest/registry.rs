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

    /// Drop the alive entry if it points to a specific entity.
    /// Used by the despawn-sweep system so an entity that got
    /// despawned through a non-`NpcDied` path doesn't leave a
    /// stale reference behind.
    pub fn forget_entity(&mut self, entity: Entity) {
        self.alive.retain(|_, e| *e != entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Id<NpcTemplate> {
        Id::<NpcTemplate>::new(s)
    }

    fn ent(n: u32) -> Entity {
        Entity::from_raw_u32(n).expect("non-zero entity index")
    }

    #[test]
    fn new_registry_has_no_templates() {
        let r = TemplateRegistry::default();
        assert!(!r.is_alive(&id("npc_sergeant")));
        assert!(!r.is_permanently_dead(&id("npc_sergeant")));
        assert!(r.entity(&id("npc_sergeant")).is_none());
    }

    #[test]
    fn register_then_is_alive() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        assert!(r.is_alive(&id("npc_sergeant")));
        assert_eq!(r.entity(&id("npc_sergeant")), Some(ent(1)));
    }

    #[test]
    fn register_overwrites_previous_entity() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.register(id("npc_sergeant"), ent(2));
        assert_eq!(r.entity(&id("npc_sergeant")), Some(ent(2)));
    }

    #[test]
    fn mark_dead_non_permanent_leaves_template_respawnable() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.mark_dead(&id("npc_sergeant"), false);
        assert!(!r.is_alive(&id("npc_sergeant")));
        assert!(!r.is_permanently_dead(&id("npc_sergeant")));
    }

    #[test]
    fn mark_dead_permanent_blocks_respawn() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.mark_dead(&id("npc_sergeant"), true);
        assert!(!r.is_alive(&id("npc_sergeant")));
        assert!(r.is_permanently_dead(&id("npc_sergeant")));
    }

    #[test]
    fn forget_entity_drops_only_matching_entry() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.register(id("npc_fixer"), ent(2));
        r.forget_entity(ent(1));
        assert!(!r.is_alive(&id("npc_sergeant")));
        assert!(r.is_alive(&id("npc_fixer")));
    }

    #[test]
    fn forget_entity_noop_when_entity_not_registered() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.forget_entity(ent(99));
        assert!(r.is_alive(&id("npc_sergeant")));
    }

    #[test]
    fn re_register_after_mark_dead_makes_alive_again() {
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.mark_dead(&id("npc_sergeant"), false);
        r.register(id("npc_sergeant"), ent(2));
        assert!(r.is_alive(&id("npc_sergeant")));
        assert_eq!(r.entity(&id("npc_sergeant")), Some(ent(2)));
    }

    #[test]
    fn permanent_dead_flag_persists_across_re_register() {
        // Permanent death is advisory — the registry itself won't
        // block `register`, but callers read `is_permanently_dead`
        // to decide. Verify the flag itself stays set.
        let mut r = TemplateRegistry::default();
        r.register(id("npc_sergeant"), ent(1));
        r.mark_dead(&id("npc_sergeant"), true);
        r.register(id("npc_sergeant"), ent(2));
        assert!(r.is_permanently_dead(&id("npc_sergeant")));
    }
}
