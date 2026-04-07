//! Looting: alive NPCs near a corpse pull items from its loadout into
//! their own general pouch.

use bevy::prelude::*;
use cordon_core::item::{ItemData, Loadout};
use cordon_data::gamedata::GameDataResource;

use crate::behavior::{CombatTarget, Dead, LootState};
use crate::components::{LoadoutComp, NpcMarker, Xp};
use crate::events::ItemLooted;
use crate::plugin::SimSet;

const LOOT_REACH: f32 = 12.0;
const LOOT_INTERVAL_SECS: f32 = 0.4;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ItemLooted>();
        app.add_systems(
            Update,
            (try_start_looting, drive_loot)
                .chain()
                .in_set(SimSet::Loot),
        );
    }
}

/// Insert a `LootState` for alive non-combat NPCs standing on a corpse.
#[allow(clippy::type_complexity)]
fn try_start_looting(
    mut commands: Commands,
    corpses: Query<(Entity, &Transform, &LoadoutComp), With<Dead>>,
    alive: Query<
        (Entity, &Transform, &CombatTarget),
        (With<NpcMarker>, Without<Dead>, Without<LootState>),
    >,
) {
    let corpse_snapshot: Vec<(Entity, Vec2)> = corpses
        .iter()
        .filter_map(|(entity, t, loadout)| {
            if loadout.0.is_empty() {
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
        let nearest = corpse_snapshot
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
#[allow(clippy::type_complexity)]
fn drive_loot(
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut commands: Commands,
    mut looted: MessageWriter<ItemLooted>,
    mut looters: Query<
        (Entity, &Xp, &CombatTarget, &mut LootState, &mut LoadoutComp),
        Without<Dead>,
    >,
    mut corpses: Query<&mut LoadoutComp, With<Dead>>,
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
                .0
                .primary
                .take()
                .or_else(|| corpse_loadout.0.secondary.take())
                .or_else(|| corpse_loadout.0.helmet.take())
                .or_else(|| corpse_loadout.0.armor.take())
                .or_else(|| corpse_loadout.0.relics.pop())
                .or_else(|| corpse_loadout.0.general.pop())
        };
        let Some(item) = item_taken else {
            commands.entity(entity).remove::<LootState>();
            continue;
        };

        let armor_data = looter_loadout
            .0
            .armor
            .as_ref()
            .and_then(|inst| items.get(&inst.def_id))
            .and_then(|def| match &def.data {
                ItemData::Armor(a) => Some(a),
                _ => None,
            });
        let capacity = Loadout::general_capacity(xp.rank(), armor_data);
        let item_id = item.def_id.clone();
        let _ = looter_loadout.0.add_to_general(item, capacity);
        looted.write(ItemLooted {
            looter: entity,
            corpse: corpse_entity,
            item: item_id,
        });
    }
}
