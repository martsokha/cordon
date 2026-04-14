//! NPC-autonomous consumable use.
//!
//! Once per minute rollover each alive NPC checks their pool
//! ratios and, if any of them crosses a need threshold, scans
//! their general carry for a matching consumable and applies it.
//! There is no player UI for this yet — every consumable use is
//! sim-driven.
//!
//! Need thresholds:
//!
//! | Pool       | Trigger          | What we look for                              |
//! |---|---|---|
//! | Health     | `< 50% max`      | a consumable with a positive `health` effect |
//! | Stamina    | `< 30% max`      | a consumable with a positive `stamina` effect |
//! | Corruption | `> 60% max`      | a consumable with a negative `corruption` effect |
//!
//! Effects flow through [`apply_or_queue`] so timed effects land
//! in `ActiveEffects` and instant effects emit `NpcPoolChanged`
//! the same way combat damage does. The relic threshold dispatcher
//! picks them up downstream without any extra wiring.

use bevy::prelude::*;
use cordon_core::item::{ItemData, ItemInstance, Loadout, ResourceTarget};
use cordon_core::primitive::{Corruption, GameTime, Health, Pool, Stamina};
use cordon_data::gamedata::GameDataResource;

use super::apply_or_queue;
use crate::behavior::combat::NpcPoolChanged;
use crate::behavior::death::Dead;
use crate::entity::npc::{ActiveEffects, NpcMarker};
use crate::resources::GameClock;

const HP_NEED_RATIO: f32 = 0.5;
const STAMINA_NEED_RATIO: f32 = 0.3;
const CORRUPTION_NEED_RATIO: f32 = 0.6;

/// Tracks the last minute we processed auto-consume for, so frames
/// that don't roll over a whole minute do nothing.
#[derive(Default)]
pub(crate) struct LastTick(Option<GameTime>);

/// Walk every alive NPC; if a need is met and a matching
/// consumable is on hand, consume one stack of it.
pub(crate) fn npc_auto_consume(
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut last: Local<LastTick>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut npcs: Query<
        (
            Entity,
            &mut Loadout,
            &mut ActiveEffects,
            &mut Pool<Health>,
            &mut Pool<Stamina>,
            &mut Pool<Corruption>,
        ),
        (With<NpcMarker>, Without<Dead>),
    >,
) {
    let now = clock.0;
    if last.0 == Some(now) {
        return;
    }
    last.0 = Some(now);

    let items = &data.0.items;

    for (entity, mut loadout, mut active, mut hp, mut stamina, mut corruption) in npcs.iter_mut() {
        // Compute needs from current pool ratios. We resolve them
        // top-down so a single tick can heal HP, then on the next
        // tick handle stamina or corruption — keeps the system
        // simple and avoids consuming three items in the same
        // minute.
        let need = pick_need(hp.current(), hp.max(), |c, m| {
            (c as f32) < (m as f32) * HP_NEED_RATIO
        })
        .map(|_| Need::Heal)
        .or_else(|| {
            pick_need(corruption.current(), corruption.max(), |c, m| {
                (c as f32) > (m as f32) * CORRUPTION_NEED_RATIO
            })
            .map(|_| Need::Scrub)
        })
        .or_else(|| {
            pick_need(stamina.current(), stamina.max(), |c, m| {
                (c as f32) < (m as f32) * STAMINA_NEED_RATIO
            })
            .map(|_| Need::Refuel)
        });
        let Some(need) = need else {
            continue;
        };

        let Some(idx) = find_matching_consumable(&loadout, items, need) else {
            continue;
        };

        // Snapshot the effects we're about to apply, then mutate
        // the pouch — applying the effects has to happen after
        // the borrow on `loadout.general` is released because
        // `apply_or_queue` doesn't take the loadout but later
        // systems do.
        let effects: Vec<_> = {
            let inst = &loadout.general[idx];
            let def = items.get(&inst.def_id).expect("def existed when picked");
            match &def.data {
                ItemData::Consumable(c) => c.effects.clone(),
                _ => unreachable!("find_matching_consumable returned non-consumable"),
            }
        };

        // Decrement the stack; drop the slot when it hits zero.
        // ItemInstance::count is u32; saturating_sub keeps the
        // pouch consistent if a future code path lets count
        // underflow into negative territory.
        let stack_empty = {
            let inst = &mut loadout.general[idx];
            inst.count = inst.count.saturating_sub(1);
            inst.count == 0
        };
        if stack_empty {
            loadout.general.swap_remove(idx);
        }

        for effect in effects {
            apply_or_queue(
                entity,
                effect,
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

/// What kind of consumable an NPC is shopping for.
#[derive(Debug, Clone, Copy)]
enum Need {
    /// Restore health (positive `health` effect).
    Heal,
    /// Scrub corruption (negative `corruption` effect).
    Scrub,
    /// Restore stamina (positive `stamina` effect).
    Refuel,
}

/// Helper that returns `Some(())` if `pred(current, max)` is true
/// and the pool has a meaningful max. Centralises the "is the
/// max even nonzero" guard so each need check stays a one-liner.
fn pick_need(current: u32, max: u32, pred: impl FnOnce(u32, u32) -> bool) -> Option<()> {
    if max == 0 {
        return None;
    }
    pred(current, max).then_some(())
}

/// Find the first general-pouch slot whose item is a consumable
/// that satisfies `need`.
fn find_matching_consumable(
    loadout: &Loadout,
    items: &std::collections::HashMap<
        cordon_core::primitive::Id<cordon_core::item::Item>,
        cordon_core::item::ItemDef,
    >,
    need: Need,
) -> Option<usize> {
    loadout.general.iter().enumerate().find_map(|(idx, inst)| {
        if !instance_satisfies(inst, items, need) {
            return None;
        }
        Some(idx)
    })
}

fn instance_satisfies(
    inst: &ItemInstance,
    items: &std::collections::HashMap<
        cordon_core::primitive::Id<cordon_core::item::Item>,
        cordon_core::item::ItemDef,
    >,
    need: Need,
) -> bool {
    let Some(def) = items.get(&inst.def_id) else {
        return false;
    };
    let ItemData::Consumable(c) = &def.data else {
        return false;
    };
    c.effects.iter().any(|e| effect_matches_need(e, need))
}

fn effect_matches_need(effect: &cordon_core::item::TimedEffect, need: Need) -> bool {
    match need {
        Need::Heal => effect.target == ResourceTarget::Health && effect.value > 0.0,
        Need::Scrub => effect.target == ResourceTarget::Corruption && effect.value < 0.0,
        Need::Refuel => effect.target == ResourceTarget::Stamina && effect.value > 0.0,
    }
}
