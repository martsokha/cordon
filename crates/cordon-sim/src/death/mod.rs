//! Death and corpse lifecycle.
//!
//! [`handle_deaths`] tags an NPC entity with [`Dead`] when its HP hits
//! zero and emits an [`NpcDied`] event for the visual layer.
//! [`cleanup_corpses`] despawns the entity once its loadout is empty
//! (fully looted) or after [`CORPSE_PERSISTENCE_MINUTES`] of game time
//! has elapsed, emitting a [`CorpseRemoved`] event so visuals can drop
//! their child meshes. [`enforce_corpse_cap`] is the hard ceiling:
//! if the time-based cleanup hasn't fired yet (e.g., long sessions in
//! which little game-time elapses), it evicts the oldest corpses
//! beyond [`MAX_DEAD_NPCS`] so entity counts stay bounded.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;

use crate::behavior::Dead;
use crate::components::{Hp, LoadoutComp, NpcMarker};
use crate::plugin::SimSet;
use crate::resources::GameClock;
use crate::tuning::{CLEANUP_INTERVAL_SECS, CORPSE_PERSISTENCE_MINUTES, MAX_DEAD_NPCS};

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

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NpcDied>();
        app.add_message::<CorpseRemoved>();
        // `handle_deaths` runs every tick so newly-dead NPCs tag in
        // the same frame combat detected the kill — visual layers and
        // event consumers expect that immediacy. The two cleanup
        // systems are pure housekeeping, so they piggy-back on the
        // squad-lifecycle throttle (1 Hz) to avoid scanning corpses
        // every frame.
        app.add_systems(
            Update,
            (
                handle_deaths,
                (cleanup_corpses, enforce_corpse_cap)
                    .chain()
                    .run_if(on_cleanup_tick),
            )
                .chain()
                .in_set(SimSet::Death),
        );
    }
}

fn on_cleanup_tick(time: Res<Time>, mut throttle: Local<f32>) -> bool {
    *throttle += time.delta_secs();
    if *throttle >= CLEANUP_INTERVAL_SECS {
        *throttle = 0.0;
        true
    } else {
        false
    }
}

/// Tag NPCs whose HP hit zero as dead and emit [`NpcDied`].
fn handle_deaths(
    clock: Res<GameClock>,
    mut commands: Commands,
    mut died: MessageWriter<NpcDied>,
    q: Query<(Entity, &Hp), (With<NpcMarker>, Without<Dead>)>,
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
fn cleanup_corpses(
    clock: Res<GameClock>,
    mut commands: Commands,
    mut removed: MessageWriter<CorpseRemoved>,
    q: Query<(Entity, &Dead, &LoadoutComp)>,
) {
    let now = clock.0;
    for (entity, dead, loadout) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = loadout.0.is_empty();
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            commands.entity(entity).despawn();
            removed.write(CorpseRemoved { entity });
        }
    }
}

/// Hard ceiling on dead NPC count. If the time-based cleanup hasn't
/// removed enough corpses to keep us under [`MAX_DEAD_NPCS`], evict
/// the oldest by `died_at`.
fn enforce_corpse_cap(
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
        commands.entity(entity).despawn();
        removed.write(CorpseRemoved { entity });
    }
}

fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
}
