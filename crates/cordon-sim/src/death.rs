//! Death and corpse lifecycle.
//!
//! [`handle_deaths`] tags an NPC entity with [`Dead`] when its HP hits
//! zero and emits an [`NpcDied`] event for the visual layer.
//! [`cleanup_corpses`] despawns the entity once its loadout is empty
//! (fully looted) or after [`CORPSE_PERSISTENCE_MINUTES`] of game time
//! has elapsed, emitting a [`CorpseRemoved`] event so visuals can drop
//! their child meshes.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;

use crate::behavior::Dead;
use crate::components::{Hp, LoadoutComp, NpcMarker};
use crate::events::{CorpseRemoved, NpcDied};
use crate::plugin::SimSet;
use crate::resources::SimWorld;

pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NpcDied>();
        app.add_message::<CorpseRemoved>();
        app.add_systems(
            Update,
            (handle_deaths, cleanup_corpses).chain().in_set(SimSet::Death),
        );
    }
}

/// Tag NPCs whose HP hit zero as dead and emit [`NpcDied`].
fn handle_deaths(
    sim: Res<SimWorld>,
    mut commands: Commands,
    mut died: MessageWriter<NpcDied>,
    q: Query<(Entity, &Hp), (With<NpcMarker>, Without<Dead>)>,
) {
    let now = sim.0.time;
    for (entity, hp) in &q {
        if hp.is_alive() {
            continue;
        }
        commands.entity(entity).insert(Dead { died_at: now });
        died.write(NpcDied {
            entity,
            killer: None,
        });
    }
}

/// Despawn corpses after the persistence window or once their loadout
/// has been fully looted.
fn cleanup_corpses(
    sim: Res<SimWorld>,
    mut commands: Commands,
    mut removed: MessageWriter<CorpseRemoved>,
    q: Query<(Entity, &Dead, &LoadoutComp)>,
) {
    let now = sim.0.time;
    for (entity, dead, loadout) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = loadout.0.is_empty();
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            commands.entity(entity).despawn();
            removed.write(CorpseRemoved { entity });
        }
    }
}

fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
}
