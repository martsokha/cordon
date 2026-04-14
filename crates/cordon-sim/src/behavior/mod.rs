//! Entity-behavior subplugins.
//!
//! Grouped here so the top-level crate has one place for "everything
//! that controls how a spawned entity acts": movement, vision,
//! combat, death, loot, squad. Each subplugin follows the
//! `{mod.rs, components.rs, systems.rs, events.rs, constants.rs}`
//! shape (with files omitted when empty), so you can find any
//! component / system / event by folder + filename.
//!
//! [`squad`] has enough internal structure (engagement, formation,
//! lifecycle, commands, behave trees) that it keeps its own feature
//! file layout alongside the canonical `components.rs` /
//! `constants.rs` files. A follow-up task will further reorganise it.

pub mod combat;
pub mod death;
pub mod effects;
pub mod loot;
pub mod movement;
pub mod squad;
pub mod vision;

use bevy::prelude::*;
use cordon_core::item::{ItemData, PassiveModifier, StatTarget};
use cordon_data::gamedata::GameDataResource;

use crate::entity::npc::{BaseMaxes, HealthPool, NpcMarker, StaminaPool};
use crate::plugin::SimSet;

/// Composer plugin that wires up every behavior subplugin.
///
/// Also owns [`sync_pool_maxes`] because it's a cross-cutting per-NPC
/// system that doesn't belong to any single subplugin — it reads
/// [`Loadout`] (from `cordon-core`) and writes the NPC's pool
/// capacities.
pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            vision::VisionPlugin,
            movement::MovementPlugin,
            combat::CombatPlugin,
            death::DeathPlugin,
            loot::LootPlugin,
            effects::EffectsPlugin,
            squad::SquadPlugin,
        ));
        // Runs in Cleanup: early enough that the rest of the frame
        // sees the updated max, late enough that the pickup from the
        // previous frame has already landed in the loadout.
        app.add_systems(Update, sync_pool_maxes.in_set(SimSet::Cleanup));
    }
}

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
        (
            &cordon_core::item::Loadout,
            &BaseMaxes,
            &mut HealthPool,
            &mut StaminaPool,
        ),
        (With<NpcMarker>, Changed<cordon_core::item::Loadout>),
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
fn apply_effective_max<K: cordon_core::primitive::PoolKind>(
    pool: &mut cordon_core::primitive::Pool<K>,
    base: u32,
    delta: i32,
) {
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
