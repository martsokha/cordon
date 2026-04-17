//! Death detection and corpse lifecycle systems.

use bevy::prelude::*;
use cordon_core::item::Loadout;
use cordon_core::primitive::{GameTime, Health, Pool};

use super::components::Dead;
use super::constants::{CLEANUP_INTERVAL_SECS, CORPSE_PERSISTENCE_MINUTES, MAX_DEAD_NPCS};
use super::events::{CorpseRemoved, NpcDied};
use crate::entity::npc::{Essential, NpcMarker};
use crate::resources::GameClock;

/// Throttle gate used by corpse-cleanup systems. Accumulates
/// `delta_secs` and fires exactly once per [`CLEANUP_INTERVAL_SECS`]
/// window.
pub(crate) fn on_cleanup_tick(
    time: Res<Time<crate::resources::Sim>>,
    mut throttle: Local<f32>,
) -> bool {
    *throttle += time.delta_secs();
    if *throttle >= CLEANUP_INTERVAL_SECS {
        *throttle = 0.0;
        true
    } else {
        false
    }
}

/// Tag NPCs whose HP hit zero as dead and emit [`NpcDied`].
/// [`Essential`] NPCs are excluded — they don't participate in
/// combat at all (filtered out of targeting and damage), so this
/// check is belt-and-suspenders.
pub fn handle_deaths(
    clock: Res<GameClock>,
    mut commands: Commands,
    mut died: MessageWriter<NpcDied>,
    q: Query<(Entity, &Pool<Health>), (With<NpcMarker>, Without<Dead>, Without<Essential>)>,
) {
    let now = clock.0;
    for (entity, hp) in &q {
        if !hp.is_empty() {
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
pub fn cleanup_corpses(
    clock: Res<GameClock>,
    mut commands: Commands,
    mut removed: MessageWriter<CorpseRemoved>,
    q: Query<(Entity, &Dead, &Loadout)>,
) {
    let now = clock.0;
    for (entity, dead, loadout) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = loadout.is_empty();
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            despawn_corpse(&mut commands, &mut removed, entity);
        }
    }
}

/// Hard ceiling on dead NPC count. If the time-based cleanup hasn't
/// removed enough corpses to keep us under [`MAX_DEAD_NPCS`], evict
/// the oldest by `died_at`.
pub fn enforce_corpse_cap(
    mut commands: Commands,
    mut removed: MessageWriter<CorpseRemoved>,
    q: Query<(Entity, &Dead)>,
) {
    let count = q.iter().count();
    if count <= MAX_DEAD_NPCS {
        return;
    }
    let mut entries: Vec<(Entity, GameTime)> = q.iter().map(|(e, d)| (e, d.died_at)).collect();
    // Oldest first.
    entries.sort_by_key(|(_, t)| to_minutes(*t));
    let to_evict = count - MAX_DEAD_NPCS;
    for (entity, _) in entries.into_iter().take(to_evict) {
        despawn_corpse(&mut commands, &mut removed, entity);
    }
}

/// Despawn a corpse entity and emit the removal event. The single
/// path through which every corpse leaves the world — new removal
/// reasons (explode, incinerate, looted-to-empty) should call into
/// here rather than duplicating the `despawn` + `write` pair.
fn despawn_corpse(
    commands: &mut Commands,
    removed: &mut MessageWriter<CorpseRemoved>,
    entity: Entity,
) {
    commands.entity(entity).despawn();
    removed.write(CorpseRemoved { entity });
}

fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
}
