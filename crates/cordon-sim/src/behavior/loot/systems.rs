//! Looting systems.

use bevy::prelude::*;
use cordon_core::item::{ItemData, Loadout};
use cordon_core::primitive::Experience;
use cordon_data::gamedata::GameDataResource;

use super::components::LootState;
use super::constants::{LOOT_INTERVAL_SECS, LOOT_REACH};
use super::events::ItemLooted;
use crate::behavior::combat::components::CombatTarget;
use crate::behavior::death::components::Dead;
use crate::entity::npc::NpcMarker;

/// Insert a `LootState` for alive non-combat NPCs standing on a corpse.
pub fn try_start_looting(
    mut commands: Commands,
    corpses: Query<(Entity, &Transform, &Loadout), With<Dead>>,
    alive: Query<
        (Entity, &Transform, &CombatTarget),
        (With<NpcMarker>, Without<Dead>, Without<LootState>),
    >,
) {
    let corpse_snapshot: Vec<(Entity, Vec2)> = corpses
        .iter()
        .filter_map(|(entity, t, loadout)| {
            if loadout.is_empty() {
                return None;
            }
            Some((entity, t.translation.truncate()))
        })
        .collect();
    if corpse_snapshot.is_empty() {
        return;
    }

    for (entity, transform, combat_target) in &alive {
        if combat_target.0.is_some() {
            continue;
        }
        let pos = transform.translation.truncate();
        let nearest =
            corpse_snapshot
                .iter()
                .filter(|(e, _)| *e != entity)
                .min_by(|(_, a), (_, b)| {
                    pos.distance_squared(*a)
                        .partial_cmp(&pos.distance_squared(*b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
        let Some((corpse_entity, corpse_pos)) = nearest else {
            continue;
        };
        if pos.distance(*corpse_pos) > LOOT_REACH {
            continue;
        }
        commands.entity(entity).insert(LootState {
            corpse: *corpse_entity,
            progress_secs: LOOT_INTERVAL_SECS,
        });
    }
}

/// Tick `LootState` and transfer one item per interval. Removes the
/// component when the corpse is empty or vanishes, or when the looter
/// starts a fight.
pub fn drive_loot(
    time: Res<Time<crate::resources::Sim>>,
    game_data: Res<GameDataResource>,
    mut commands: Commands,
    mut looted: MessageWriter<ItemLooted>,
    mut looters: Query<
        (
            Entity,
            &Experience,
            &CombatTarget,
            &mut LootState,
            &mut Loadout,
        ),
        Without<Dead>,
    >,
    mut corpses: Query<&mut Loadout, With<Dead>>,
) {
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    for (entity, xp, combat_target, mut loot_state, mut looter_loadout) in &mut looters {
        if combat_target.0.is_some() {
            commands.entity(entity).remove::<LootState>();
            continue;
        }

        let corpse_entity = loot_state.corpse;
        if corpses.get(corpse_entity).is_err() {
            commands.entity(entity).remove::<LootState>();
            continue;
        }

        loot_state.progress_secs -= dt;
        if loot_state.progress_secs > 0.0 {
            continue;
        }
        loot_state.progress_secs = LOOT_INTERVAL_SECS;

        let item_taken = {
            let Ok(mut corpse_loadout) = corpses.get_mut(corpse_entity) else {
                commands.entity(entity).remove::<LootState>();
                continue;
            };
            corpse_loadout
                .primary
                .take()
                .or_else(|| corpse_loadout.secondary.take())
                .or_else(|| corpse_loadout.helmet.take())
                .or_else(|| corpse_loadout.armor.take())
                .or_else(|| corpse_loadout.relics.pop())
                .or_else(|| corpse_loadout.general.pop())
        };
        let Some(item) = item_taken else {
            commands.entity(entity).remove::<LootState>();
            continue;
        };

        let armor_data = looter_loadout
            .armor
            .as_ref()
            .and_then(|inst| items.get(&inst.def_id))
            .and_then(|def| match &def.data {
                ItemData::Armor(a) => Some(a),
                _ => None,
            });
        let capacity = Loadout::general_capacity(xp.npc_rank(), armor_data);
        let item_id = item.def_id.clone();
        let _ = looter_loadout.add_to_general(item, capacity);
        looted.write(ItemLooted {
            looter: entity,
            corpse: corpse_entity,
            item: item_id,
        });
    }
}
