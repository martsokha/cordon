//! Per-NPC physical and combat state, plus the per-NPC movement system.
//!
//! These components are written by the squad/combat/loot systems and
//! read by the per-NPC drivers below. There is no state machine —
//! interruptions are just "set a new target, drop the old one", which
//! is naturally race-free under explicit system ordering.

use bevy::prelude::*;
use cordon_core::item::{ItemData, Loadout, PassiveModifier, StatTarget};
use cordon_core::primitive::{GameTime, Rank};
use cordon_data::gamedata::GameDataResource;

use crate::components::{BaseMaxes, Hp, HungerPool, NpcMarker, StaminaPool};
use crate::plugin::SimSet;
use crate::tuning::MAP_BOUND;

/// The point this NPC is currently walking toward, in world space.
/// `None` means the NPC is standing still.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct MovementTarget(pub Option<Vec2>);

/// How fast this NPC walks toward [`MovementTarget`], in map units per
/// second. Updated by the system that sets the target.
#[derive(Component, Debug, Clone, Copy)]
pub struct MovementSpeed(pub f32);

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(30.0)
    }
}

/// The hostile NPC this entity is firing on. `None` means the NPC is
/// not currently engaged in combat. The squad engagement scanner sets
/// this; the combat firing system reads it.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct CombatTarget(pub Option<Entity>);

/// Per-NPC firing state: cooldown until next shot.
///
/// Reload is not modelled as a timed phase — magazines refill
/// instantly from the general pouch when empty, and fire tempo is
/// controlled entirely by the weapon's `fire_rate`.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct FireState {
    pub cooldown_secs: f32,
}

/// Per-NPC looting progress. Present only while the NPC is actively
/// looting a specific corpse. Removed when the corpse is empty or the
/// NPC walks away.
#[derive(Component, Debug, Clone, Copy)]
pub struct LootState {
    pub corpse: Entity,
    pub progress_secs: f32,
}

/// Vision radius (in map units) for spotting hostiles.
#[derive(Component, Debug, Clone, Copy)]
pub struct Vision {
    pub radius: f32,
}

impl Vision {
    /// Default vision: 120 base + 15 per rank tier above Novice.
    pub fn for_npc(rank: Rank) -> Self {
        let radius = 120.0 + (rank.tier() as f32 - 1.0) * 15.0;
        Self { radius }
    }
}

/// Marker for anomaly entities, contributing to LOS blocking. Spawned
/// by the visual layer when it lays out the map; the combat system
/// reads `(Transform, AnomalyZone)` to compute LOS.
#[derive(Component, Debug, Clone, Copy)]
pub struct AnomalyZone {
    pub radius: f32,
}

/// Marker for a corpse with its time of death. Inserted by the death
/// system when an NPC's HP hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
}

/// Walk every NPC with a [`MovementTarget`] toward that point.
///
/// Filters on `MovementTarget` so NPCs that aren't moving don't even
/// touch their transform — Bevy's change detection skips them and the
/// downstream transform-propagation system has less work.
pub fn move_npcs(
    time: Res<Time>,
    mut q: Query<(&MovementTarget, &MovementSpeed, &mut Transform), Without<Dead>>,
) {
    let dt = time.delta_secs();
    for (target, speed, mut transform) in &mut q {
        let Some(target) = target.0 else { continue };
        let pos = transform.translation.truncate();
        let delta = target - pos;
        let dist = delta.length();
        if dist < 0.5 {
            continue;
        }
        let dir = delta / dist;
        let step = (speed.0 * dt).min(dist);
        transform.translation.x += dir.x * step;
        transform.translation.y += dir.y * step;
        transform.translation.x = transform.translation.x.clamp(-MAP_BOUND, MAP_BOUND);
        transform.translation.y = transform.translation.y.clamp(-MAP_BOUND, MAP_BOUND);
    }
}

/// Recompute each NPC's pool maximums from their `BaseMaxes` plus
/// any relic passive modifiers targeting `MaxHealth` / `MaxStamina`
/// / `MaxHunger`.
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
            &Loadout,
            &BaseMaxes,
            &mut Hp,
            &mut StaminaPool,
            &mut HungerPool,
        ),
        (With<NpcMarker>, Changed<Loadout>),
    >,
) {
    let items = &game_data.0.items;
    for (loadout, base, mut hp, mut stamina, mut hunger) in &mut changed {
        // Sum each stat's contribution across all equipped relics.
        let mut dmax_hp: i32 = 0;
        let mut dmax_stamina: i32 = 0;
        let mut dmax_hunger: i32 = 0;
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
                    StatTarget::MaxHunger => dmax_hunger += v,
                    _ => {}
                }
            }
        }

        apply_effective_max(&mut hp, base.hp, dmax_hp);
        apply_effective_max(&mut stamina, base.stamina, dmax_stamina);
        apply_effective_max(&mut hunger, base.hunger, dmax_hunger);
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

/// Plugin registering per-NPC systems: movement, pool-max sync, etc.
pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_npcs.in_set(SimSet::Movement));
        // Runs in Cleanup: early enough that the rest of the frame
        // sees the updated max, late enough that the pickup from the
        // previous frame has already landed in the loadout.
        app.add_systems(Update, sync_pool_maxes.in_set(SimSet::Cleanup));
    }
}
