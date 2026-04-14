//! Equipment-side-effect systems.

use bevy::prelude::*;
use cordon_core::item::{ItemData, Loadout, PassiveModifier, StatTarget};
use cordon_core::primitive::{Health, Pool, PoolKind, Stamina};
use cordon_data::gamedata::GameDataResource;

use crate::entity::npc::{BaseMaxes, NpcMarker};

/// Recompute each NPC's pool maximums from their `BaseMaxes` plus
/// any relic passive modifiers targeting `MaxHealth` / `MaxStamina`.
///
/// Runs on `Changed<Loadout>` so we only pay the cost for NPCs
/// whose equipment just mutated. Keeping the base in a separate
/// component means drops (future work) reverse cleanly — the
/// system just recomputes from base + whatever's left equipped.
///
/// If a pool's effective max shrinks below its current, the
/// current clamps down (`Pool::set_max` handles that). If the
/// max grows, we restore the delta so picking up a `+10 MaxHP`
/// relic feels like "you gained 10 HP right now", not "you earned
/// 10 HP worth of headroom".
pub fn sync_pool_maxes(
    game_data: Res<GameDataResource>,
    mut changed: Query<
        (&Loadout, &BaseMaxes, &mut Pool<Health>, &mut Pool<Stamina>),
        (With<NpcMarker>, Changed<Loadout>),
    >,
) {
    let items = &game_data.0.items;
    for (loadout, base, mut hp, mut stamina) in &mut changed {
        // Sum each stat's contribution across all equipped relics.
        let mut dmax_hp: i32 = 0;
        let mut dmax_stamina: i32 = 0;
        for inst in &loadout.relics {
            let Some(def) = items.get(&inst.def_id) else {
                continue;
            };
            let ItemData::Relic(relic) = &def.data else {
                continue;
            };
            for PassiveModifier { target, value } in &relic.passive {
                let v = value.round() as i32;
                match target {
                    StatTarget::MaxHealth => dmax_hp += v,
                    StatTarget::MaxStamina => dmax_stamina += v,
                    _ => {}
                }
            }
        }

        apply_effective_max(&mut hp, base.hp, dmax_hp);
        apply_effective_max(&mut stamina, base.stamina, dmax_stamina);
    }
}

/// Update a pool's max to `base + delta` (clamped to non-negative).
/// If the max grew, `restore` the delta so the pickup feels
/// rewarding. If it shrank, `set_max` clamps current down.
fn apply_effective_max<K: PoolKind>(pool: &mut Pool<K>, base: u32, delta: i32) {
    let new_max = (base as i64 + delta as i64).max(0) as u32;
    let old_max = pool.max();
    if new_max == old_max {
        return;
    }
    pool.set_max(new_max);
    if new_max > old_max {
        pool.restore(new_max - old_max);
    }
}
