//! Throwable impact path + NPC grenade AI.
//!
//! Two halves:
//!
//! - [`npc_throw_grenades`] picks an NPC with a throwable in
//!   their general pouch and a current combat target, then writes
//!   a [`ThrowableImpact`] message at the target's position.
//! - [`process_throwable_impacts`] reads those messages, finds
//!   every alive entity inside the throwable's `aoe` (or the
//!   nearest single target if `aoe.is_none()`), filters by
//!   "hostile to thrower", and applies each effect via
//!   [`apply_or_queue`] so the unified pool-event bus picks up
//!   the resulting changes.
//!
//! Throwables produce no `OnHit` against the thrower's allies —
//! the hostile filter is the friendly-fire guard. Self-throwing
//! is allowed only if the thrower is in their own enemy list,
//! which the relation table will never report.
//!
//! Throw cadence is gated by a per-thrower one-minute cooldown
//! ([`GrenadeCooldown`]) so a single fight doesn't drain a
//! pouch in one tick.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::item::{ItemData, Loadout};
use cordon_core::primitive::{Corruption, GameTime, Health, Id, Pool, Stamina};
use cordon_data::gamedata::GameDataResource;

use super::apply_or_queue;
use crate::behavior::combat::{CombatTarget, NpcPoolChanged, is_hostile};
use crate::behavior::death::Dead;
use crate::entity::npc::{ActiveEffects, FactionId, NpcMarker};
use crate::resources::GameClock;

/// Minimum minutes between two throws from the same thrower.
const GRENADE_COOLDOWN_MINUTES: u32 = 1;

/// One throwable detonation, written by [`npc_throw_grenades`]
/// and consumed by [`process_throwable_impacts`].
///
/// `item` is the *def id* of the throwable, not an item instance —
/// the impact processor looks the def up in the catalog rather
/// than carrying the whole effect list around in the message
/// payload. Authored quest scripts can also write this message
/// directly to script a scripted boom.
#[derive(Message, Debug, Clone)]
pub struct ThrowableImpact {
    /// The entity that threw the grenade. May be `None` for
    /// scripted impacts that don't have a real thrower.
    pub thrower: Option<Entity>,
    /// World-space impact point.
    pub target_pos: Vec2,
    /// Catalog id of the throwable item.
    pub item: Id<cordon_core::item::Item>,
}

/// Per-NPC throw cooldown. Inserted lazily on first throw.
#[derive(Component, Debug, Clone, Copy)]
pub struct GrenadeCooldown {
    pub last_throw: GameTime,
}

/// Walk every NPC with a current combat target; if they have a
/// throwable in their general pouch and the cooldown has elapsed,
/// write a [`ThrowableImpact`] at their target's position and
/// decrement the throwable stack.
pub(crate) fn npc_throw_grenades(
    mut commands: Commands,
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut throws: MessageWriter<ThrowableImpact>,
    target_positions: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut throwers: Query<
        (
            Entity,
            &CombatTarget,
            &mut Loadout,
            Option<&GrenadeCooldown>,
        ),
        (With<NpcMarker>, Without<Dead>),
    >,
) {
    let now = clock.0;
    let items = &data.0.items;

    for (thrower, combat, mut loadout, cooldown) in throwers.iter_mut() {
        let Some(target) = combat.0 else {
            continue;
        };
        if let Some(cd) = cooldown
            && now.minutes_since(cd.last_throw) < GRENADE_COOLDOWN_MINUTES
        {
            continue;
        }
        let Ok(target_transform) = target_positions.get(target) else {
            continue;
        };

        // Find the first throwable in the general pouch.
        let Some(idx) = loadout.general.iter().position(|inst| {
            items
                .get(&inst.def_id)
                .map(|def| matches!(&def.data, ItemData::Throwable(_)))
                .unwrap_or(false)
        }) else {
            continue;
        };

        let item_id = loadout.general[idx].def_id.clone();

        // Decrement the stack; drop the slot when it hits zero.
        let stack_empty = {
            let inst = &mut loadout.general[idx];
            inst.count = inst.count.saturating_sub(1);
            inst.count == 0
        };
        if stack_empty {
            loadout.general.swap_remove(idx);
        }

        throws.write(ThrowableImpact {
            thrower: Some(thrower),
            target_pos: target_transform.translation.truncate(),
            item: item_id,
        });
        commands
            .entity(thrower)
            .insert(GrenadeCooldown { last_throw: now });
    }
}

/// Apply [`ThrowableImpact`] messages to every hostile entity
/// inside the impact's effect radius.
///
/// Walks the catalog once per impact to read the throwable's
/// effect list, then sweeps the alive-NPC query for victims.
/// Each effect is fed through [`apply_or_queue`], so timed
/// effects land in [`ActiveEffects`] and instant effects emit
/// [`NpcPoolChanged`] which the relic dispatcher picks up.
pub(crate) fn process_throwable_impacts(
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut impacts: MessageReader<ThrowableImpact>,
    thrower_factions: Query<&FactionId, (With<NpcMarker>, Without<Dead>)>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut victims: Query<
        (
            Entity,
            &Transform,
            &FactionId,
            &mut ActiveEffects,
            &mut Pool<Health>,
            &mut Pool<Stamina>,
            &mut Pool<Corruption>,
        ),
        (With<NpcMarker>, Without<Dead>),
    >,
) {
    let now = clock.0;
    let items = &data.0.items;
    let factions = &data.0.factions;

    for impact in impacts.read() {
        let Some(def) = items.get(&impact.item) else {
            continue;
        };
        let ItemData::Throwable(throwable) = &def.data else {
            continue;
        };
        if throwable.effects.is_empty() {
            continue;
        }

        // Determine the thrower's faction so we can filter
        // friendly fire. Scripted impacts (no thrower) treat
        // everyone as hostile, which lets quest scripts model
        // an environment hazard with the same code path.
        let thrower_faction: Option<Id<Faction>> = impact
            .thrower
            .and_then(|t| thrower_factions.get(t).ok())
            .map(|f| f.0.clone());

        // Pick affected entities. With aoe, every entity inside
        // the radius is a victim; without, the single nearest.
        let aoe_sq = throwable
            .effects
            .iter()
            .filter_map(|e| e.aoe.map(|d| d.value()))
            .fold(0.0_f32, f32::max);
        let aoe_sq = aoe_sq * aoe_sq;

        // Snapshot which entities to hit before mutating, so we
        // don't overlap mutable borrows during the apply pass.
        let mut hits: Vec<Entity> = Vec::new();
        if aoe_sq > 0.0 {
            for (entity, transform, faction, ..) in victims.iter() {
                if !hostile_or_unowned(thrower_faction.as_ref(), &faction.0, factions) {
                    continue;
                }
                let pos = transform.translation.truncate();
                if pos.distance_squared(impact.target_pos) <= aoe_sq {
                    hits.push(entity);
                }
            }
        } else {
            // Single-target: nearest hostile within a fixed reach.
            const SINGLE_TARGET_REACH_SQ: f32 = 25.0 * 25.0;
            let mut best: Option<(Entity, f32)> = None;
            for (entity, transform, faction, ..) in victims.iter() {
                if !hostile_or_unowned(thrower_faction.as_ref(), &faction.0, factions) {
                    continue;
                }
                let pos = transform.translation.truncate();
                let d_sq = pos.distance_squared(impact.target_pos);
                if d_sq > SINGLE_TARGET_REACH_SQ {
                    continue;
                }
                if best.map(|(_, b)| d_sq < b).unwrap_or(true) {
                    best = Some((entity, d_sq));
                }
            }
            if let Some((e, _)) = best {
                hits.push(e);
            }
        }

        for victim in hits {
            let Ok((entity, _, _, mut active, mut hp, mut stamina, mut corruption)) =
                victims.get_mut(victim)
            else {
                continue;
            };
            for effect in &throwable.effects {
                apply_or_queue(
                    entity,
                    *effect,
                    now,
                    &mut active,
                    &mut hp,
                    &mut stamina,
                    &mut corruption,
                    &mut pool_changed,
                );
            }
        }
    }
}

/// True if `victim` is hostile to `thrower`. Scripted impacts
/// (no thrower) treat every faction as hostile so the impact
/// reads as an environmental boom.
fn hostile_or_unowned(
    thrower: Option<&Id<Faction>>,
    victim: &Id<Faction>,
    factions: &std::collections::HashMap<Id<Faction>, cordon_core::entity::faction::FactionDef>,
) -> bool {
    match thrower {
        None => true,
        Some(t) => is_hostile(t, victim, factions),
    }
}
