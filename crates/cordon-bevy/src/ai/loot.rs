//! Looting: alive NPCs near a corpse pull items from its loadout into
//! their own general pouch.
//!
//! [`try_start_looting`] watches for non-fighting NPCs that pass within
//! [`LOOT_REACH`] of a non-empty corpse and inserts a [`LootState`]
//! component. [`drive_loot`] then ticks the per-item progress timer
//! and, whenever an interval elapses, transfers one item from the
//! corpse to the looter (skipping if the looter's pouch is full).

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::item::{ItemData, Loadout};
use cordon_core::primitive::Uid;
use cordon_data::gamedata::GameDataResource;

use super::AiSet;
use super::behavior::{CombatTarget, LootState};
use super::death::Dead;
use crate::PlayingState;
use crate::laptop::NpcDot;
use crate::world::SimWorld;

/// Distance under which an alive NPC will start looting an adjacent corpse.
const LOOT_REACH: f32 = 12.0;
/// How long a single loot transfer takes (seconds per item).
const LOOT_INTERVAL_SECS: f32 = 0.4;

/// Plugin registering the loot systems.
pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (try_start_looting, drive_loot)
                .chain()
                .in_set(AiSet::Loot)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Insert a `LootState` for alive non-combat NPCs standing on a corpse.
#[allow(clippy::type_complexity)]
fn try_start_looting(
    sim: Option<Res<SimWorld>>,
    mut commands: Commands,
    corpses: Query<(&NpcDot, &Transform), With<Dead>>,
    alive: Query<
        (Entity, &NpcDot, &Transform, &CombatTarget),
        (Without<Dead>, Without<LootState>),
    >,
) {
    let Some(sim) = sim else { return };

    // Snapshot non-empty corpses with their positions.
    let corpse_snapshot: Vec<(Uid<Npc>, Vec2)> = corpses
        .iter()
        .filter_map(|(dot, t)| {
            let npc = sim.0.npcs.get(&dot.uid)?;
            if npc.loadout.is_empty() {
                return None;
            }
            Some((dot.uid, t.translation.truncate()))
        })
        .collect();
    if corpse_snapshot.is_empty() {
        return;
    }

    for (entity, looter_dot, transform, combat_target) in &alive {
        // Don't pre-empt fighting.
        if combat_target.0.is_some() {
            continue;
        }
        let pos = transform.translation.truncate();
        let nearest = corpse_snapshot
            .iter()
            .filter(|(uid, _)| *uid != looter_dot.uid)
            .min_by(|(_, a), (_, b)| {
                pos.distance_squared(*a)
                    .partial_cmp(&pos.distance_squared(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let Some((corpse_uid, corpse_pos)) = nearest else {
            continue;
        };
        if pos.distance(*corpse_pos) > LOOT_REACH {
            continue;
        }
        commands.entity(entity).insert(LootState {
            corpse: *corpse_uid,
            progress_secs: LOOT_INTERVAL_SECS,
        });
    }
}

/// Drive `LootState`: tick the progress timer, transfer one item per
/// interval. Removes the component when the corpse is empty or
/// vanishes, or when the looter starts a fight.
#[allow(clippy::type_complexity)]
fn drive_loot(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut commands: Commands,
    mut looters: Query<
        (Entity, &NpcDot, &CombatTarget, &mut LootState),
        Without<Dead>,
    >,
    corpses: Query<&NpcDot, With<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    let corpse_uids: HashMap<Uid<Npc>, ()> =
        corpses.iter().map(|dot| (dot.uid, ())).collect();

    let mut transfers: Vec<(NpcDot, Uid<Npc>, Entity)> = Vec::new();

    for (entity, looter_dot, combat_target, mut loot_state) in &mut looters {
        // Combat takes priority — drop loot if we picked up a fight.
        if combat_target.0.is_some() {
            commands.entity(entity).remove::<LootState>();
            continue;
        }
        // Corpse vanished — drop loot.
        if !corpse_uids.contains_key(&loot_state.corpse) {
            commands.entity(entity).remove::<LootState>();
            continue;
        }

        loot_state.progress_secs -= dt;
        if loot_state.progress_secs > 0.0 {
            continue;
        }
        loot_state.progress_secs = LOOT_INTERVAL_SECS;
        transfers.push((*looter_dot, loot_state.corpse, entity));
    }

    for (looter_dot, corpse_uid, looter_entity) in transfers {
        // Pop one item from the corpse in priority order.
        let item_taken = {
            let Some(corpse) = sim.0.npcs.get_mut(&corpse_uid) else {
                continue;
            };
            corpse
                .loadout
                .primary
                .take()
                .or_else(|| corpse.loadout.secondary.take())
                .or_else(|| corpse.loadout.helmet.take())
                .or_else(|| corpse.loadout.armor.take())
                .or_else(|| corpse.loadout.relics.pop())
                .or_else(|| corpse.loadout.general.pop())
        };
        let Some(item) = item_taken else {
            // Corpse is empty — stop looting.
            commands.entity(looter_entity).remove::<LootState>();
            continue;
        };

        if let Some(looter) = sim.0.npcs.get_mut(&looter_dot.uid) {
            // Compute capacity from rank + equipped armor.
            let armor_data = looter
                .loadout
                .armor
                .as_ref()
                .and_then(|inst| items.get(&inst.def_id))
                .and_then(|def| match &def.data {
                    ItemData::Armor(a) => Some(a),
                    _ => None,
                });
            let capacity = Loadout::general_capacity(looter.rank(), armor_data);
            let _ = looter.loadout.add_to_general(item, capacity);
        }
    }
}
