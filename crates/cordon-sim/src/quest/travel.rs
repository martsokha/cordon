//! Template NPC bunker-travel arrival detection.
//!
//! Template NPCs spawned for a quest `Talk` stage walk from a
//! random faction settlement to the bunker before being enqueued
//! as a visitor. This module owns the arrival message and the
//! system that fires it when a traveling NPC gets close enough
//! to the bunker dot.

use bevy::prelude::*;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::primitive::Id;
use cordon_core::world::BUNKER_MAP_POS;

use super::registry::TemplateRegistry;
use crate::entity::npc::{SpawnOrigin, TemplateId, TravelingHome, TravelingToBunker};

/// Distance at which a traveling NPC is considered "at the bunker."
const ARRIVAL_RADIUS: f32 = 15.0;

/// Fired when a template NPC reaches the bunker. Handled by the
/// Bevy layer to enqueue a visitor + hide the map dot.
#[derive(Message, Debug, Clone)]
pub struct BunkerArrival {
    pub entity: Entity,
    pub template: Id<NpcTemplate>,
}

pub fn detect_bunker_arrival(
    mut commands: Commands,
    traveling_q: Query<(Entity, &Transform, &TemplateId), With<TravelingToBunker>>,
    mut arrivals: MessageWriter<BunkerArrival>,
) {
    for (entity, transform, tmpl) in &traveling_q {
        let pos = transform.translation.truncate();
        if pos.distance(BUNKER_MAP_POS) < ARRIVAL_RADIUS {
            // Remove the travel marker immediately so subsequent
            // ticks of this system don't re-fire the message
            // before the downstream handler has finished. Commands
            // flush between frames, so the `With<TravelingToBunker>`
            // filter skips this entity on the next run.
            commands.entity(entity).remove::<TravelingToBunker>();
            arrivals.write(BunkerArrival {
                entity,
                template: tmpl.0.clone(),
            });
        }
    }
}

/// Fired when a template NPC, having finished dialogue, reaches
/// the settlement it originally spawned at. Handled by the Bevy
/// layer to despawn the entity and update the registry.
#[derive(Message, Debug, Clone)]
pub struct HomeArrival {
    pub entity: Entity,
    pub template: Id<NpcTemplate>,
}

pub fn detect_home_arrival(
    mut commands: Commands,
    traveling_q: Query<(Entity, &Transform, &TemplateId, &SpawnOrigin), With<TravelingHome>>,
    mut arrivals: MessageWriter<HomeArrival>,
) {
    for (entity, transform, tmpl, origin) in &traveling_q {
        let pos = transform.translation.truncate();
        if pos.distance(origin.0) < ARRIVAL_RADIUS {
            commands.entity(entity).remove::<TravelingHome>();
            arrivals.write(HomeArrival {
                entity,
                template: tmpl.0.clone(),
            });
        }
    }
}

/// Defensive sweep: when a [`TemplateId`] component is removed from
/// an entity (e.g. because the entity was despawned through any
/// path that bypasses [`NpcDied`]), drop the stale `alive` entry
/// from the registry. Without this, future `SpawnNpc` requests
/// could see `is_alive == true` for a dead entity handle.
pub fn prune_despawned_templates(
    mut removed: RemovedComponents<TemplateId>,
    mut registry: ResMut<TemplateRegistry>,
) {
    for entity in removed.read() {
        registry.forget_entity(entity);
    }
}
