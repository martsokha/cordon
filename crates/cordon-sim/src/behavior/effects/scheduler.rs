//! Periodic-trigger scheduler.
//!
//! Every equipped relic with a [`EffectTrigger::Periodic`] trigger
//! gets a [`PeriodicEntry`] in the global [`PeriodicTriggers`]
//! resource, scheduled to fire every
//! [`PERIODIC_INTERVAL_MINUTES`] minutes of game time. One global
//! table rather than per-entity state because at steady state the
//! total scheduled entries (population × relic slots) stays in the
//! low hundreds and a plain `Vec<Entry>` is easier to reason about
//! than components with insertion/removal on loadout change.
//!
//! Two systems:
//! - [`sync_periodic_triggers`] rebuilds entries for entities whose
//!   loadout changed this frame, and drops entries whose carrier
//!   despawned.
//! - [`fire_periodic_triggers`] applies every entry whose
//!   `next_fire` has landed and advances the schedule past `now`.

use bevy::prelude::*;
use cordon_core::item::{
    EffectTrigger, Item, ItemData, Loadout, PERIODIC_INTERVAL_MINUTES, TimedEffect,
};
use cordon_core::primitive::{Corruption, GameTime, Health, Id, Pool, Stamina};
use cordon_data::gamedata::GameDataResource;

use super::apply::apply_or_queue;
use crate::behavior::combat::NpcPoolChanged;
use crate::behavior::death::Dead;
use crate::entity::npc::{ActiveEffects, NpcMarker};
use crate::resources::GameClock;

/// Scheduled `Periodic` relic triggers across every NPC.
///
/// A single global resource rather than per-entity state because
/// the table is small (population × relic slots ≈ hundreds of
/// entries max at steady state) and a plain `Vec<Entry>` is
/// easier to reason about than a component with insertion /
/// removal on loadout change.
#[derive(Resource, Debug, Default)]
pub struct PeriodicTriggers {
    entries: Vec<PeriodicEntry>,
}

/// One scheduled periodic trigger. Every periodic relic fires on
/// the same fixed cadence ([`PERIODIC_INTERVAL_MINUTES`]), so the
/// entry doesn't store its own period.
#[derive(Debug, Clone)]
struct PeriodicEntry {
    /// Which entity carries the relic.
    entity: Entity,
    /// Which relic produced this trigger. Used to prune on
    /// loadout change.
    relic: Id<Item>,
    /// The effect to apply on each fire.
    effect: TimedEffect,
    /// When the next fire is scheduled. Compared against
    /// `GameClock::now` each tick.
    next_fire: GameTime,
}

/// Walk every entity with a changed loadout and rebuild its
/// periodic-trigger entries in [`PeriodicTriggers`].
///
/// Two operations in one pass:
/// 1. Drop every existing entry whose entity+relic combination is
///    no longer equipped (unequipped relic → no fires).
/// 2. Add an entry for every equipped relic's `Periodic` trigger
///    that isn't already scheduled.
///
/// Change detection on [`Loadout`] means this only runs when
/// equipment actually shifts — quiet in the common case.
///
/// Dead-entity pruning uses `RemovedComponents<NpcMarker>` as the
/// signal rather than scanning every dead entity each frame: when
/// `NpcMarker` comes off an entity (on despawn, which takes the
/// whole bundle with it), the removed-components reader yields
/// exactly those entities this frame, and we drop their periodic
/// entries.
pub(super) fn sync_periodic_triggers(
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    changed: Query<(Entity, &Loadout), (With<NpcMarker>, Changed<Loadout>, Without<Dead>)>,
    mut removed_npcs: RemovedComponents<NpcMarker>,
) {
    // Drop any entries whose carrier despawned this frame. Cheap:
    // the reader yields only removed entities, not the whole
    // alive-NPC set.
    let removed: Vec<Entity> = removed_npcs.read().collect();
    if !removed.is_empty() {
        periodic.entries.retain(|e| !removed.contains(&e.entity));
    }

    if changed.is_empty() {
        return;
    }
    let now = clock.0;
    let items = &data.0.items;

    // For every entity whose loadout changed this frame, build the
    // set of `(relic_id, effect)` pairs the entity currently
    // carries for periodic triggers. Any existing entry for this
    // entity that isn't in the set gets dropped; any pair not yet
    // in the entries gets added.
    for (entity, loadout) in &changed {
        let mut carried: Vec<(Id<Item>, TimedEffect)> = Vec::new();
        for relic_instance in &loadout.relics {
            let Some(def) = items.get(&relic_instance.def_id) else {
                continue;
            };
            let ItemData::Relic(relic) = &def.data else {
                continue;
            };
            for triggered in &relic.triggered {
                if triggered.trigger == EffectTrigger::Periodic {
                    carried.push((relic_instance.def_id.clone(), triggered.effect));
                }
            }
        }

        // Drop stale entries for this entity.
        periodic.entries.retain(|e| {
            if e.entity != entity {
                return true;
            }
            carried.iter().any(|(relic, _)| e.relic == *relic)
        });

        // Add entries that don't exist yet.
        for (relic, effect) in carried {
            let already = periodic
                .entries
                .iter()
                .any(|e| e.entity == entity && e.relic == relic);
            if already {
                continue;
            }
            let mut next_fire = now;
            next_fire.advance_minutes(PERIODIC_INTERVAL_MINUTES);
            periodic.entries.push(PeriodicEntry {
                entity,
                relic,
                effect,
                next_fire,
            });
        }
    }
}

/// Fire every periodic entry whose `next_fire` has landed.
///
/// Single pass: apply the effect, then advance `next_fire` to the
/// first minute strictly after `now`. The advance loop handles
/// clock stutter where multiple periods elapsed in one frame by
/// skipping the backlog (fire once, not N times).
pub(super) fn fire_periodic_triggers(
    clock: Res<GameClock>,
    mut periodic: ResMut<PeriodicTriggers>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut carriers: Query<
        (
            &mut ActiveEffects,
            &mut Pool<Health>,
            &mut Pool<Stamina>,
            &mut Pool<Corruption>,
        ),
        Without<Dead>,
    >,
) {
    let now = clock.0;
    for entry in periodic.entries.iter_mut() {
        if !is_due(now, entry.next_fire) {
            continue;
        }
        if let Ok((mut active, mut hp, mut stamina, mut corruption)) =
            carriers.get_mut(entry.entity)
        {
            apply_or_queue(
                entry.entity,
                entry.effect,
                now,
                &mut active,
                &mut hp,
                &mut stamina,
                &mut corruption,
                &mut pool_changed,
            );
        }
        // Advance next_fire until it lands strictly in the future.
        // Dead-carrier entries are left in place and cleaned up by
        // `sync_periodic_triggers` when `RemovedComponents<NpcMarker>`
        // fires on despawn.
        while is_due(now, entry.next_fire) {
            entry.next_fire.advance_minutes(PERIODIC_INTERVAL_MINUTES);
        }
    }
}

/// True if `now >= scheduled`. Hoisted so the fire check and the
/// advance loop read off the same predicate.
fn is_due(now: GameTime, scheduled: GameTime) -> bool {
    now.minutes_since(scheduled) > 0 || now == scheduled
}
