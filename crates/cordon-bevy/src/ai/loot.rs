//! Looting: alive NPCs near a corpse pull items from its loadout into
//! their own general pouch.
//!
//! [`try_start_looting`] watches for non-fighting NPCs that pass within
//! [`LOOT_REACH`] of a non-empty corpse and pushes [`Action::Loot`].
//! [`drive_loot_actions`] then ticks the per-item progress timer and,
//! whenever an interval elapses, transfers one item from the corpse to
//! the looter (skipping if the looter's pouch is full).

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::item::{ItemData, Loadout};
use cordon_core::primitive::Uid;
use cordon_data::gamedata::GameDataResource;
use moonshine_behavior::prelude::*;

use super::behavior::Action;
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
            (try_start_looting, drive_loot_actions)
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Push `Action::Loot` for alive NPCs that are standing near a corpse
/// and not currently engaged in combat.
#[allow(clippy::type_complexity)]
fn try_start_looting(
    sim: Option<Res<SimWorld>>,
    corpses: Query<(&NpcDot, &Transform), With<Dead>>,
    mut alive: Query<(&NpcDot, &Transform, BehaviorMut<Action>), Without<Dead>>,
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

    for (looter_dot, transform, mut behavior) in &mut alive {
        // Don't pre-empt fighting or already-looting.
        if matches!(
            behavior.current(),
            Action::Engage { .. } | Action::Loot { .. } | Action::Flee { .. }
        ) {
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
        let _ = behavior.try_start(Action::Loot {
            target: *corpse_uid,
            progress_secs: LOOT_INTERVAL_SECS,
        });
    }
}

/// Drive `Action::Loot`: tick the progress timer, transfer items.
#[allow(clippy::type_complexity)]
fn drive_loot_actions(
    sim: Option<ResMut<SimWorld>>,
    time: Res<Time>,
    game_data: Res<GameDataResource>,
    mut q: Query<(&NpcDot, BehaviorMut<Action>), Without<Dead>>,
    corpses: Query<&NpcDot, With<Dead>>,
) {
    let Some(mut sim) = sim else { return };
    let items = &game_data.0.items;
    let dt = time.delta_secs();

    let corpse_uids: HashMap<Uid<Npc>, ()> = corpses.iter().map(|dot| (dot.uid, ())).collect();

    let mut transfers: Vec<(NpcDot, NpcDot)> = Vec::new();

    for (looter_dot, mut behavior) in &mut q {
        let target_uid = match behavior.current_mut() {
            Action::Loot {
                target,
                progress_secs,
            } => {
                *progress_secs -= dt;
                if *progress_secs > 0.0 {
                    continue;
                }
                *progress_secs = LOOT_INTERVAL_SECS;
                *target
            }
            _ => continue,
        };

        if !corpse_uids.contains_key(&target_uid) {
            // Corpse vanished — bail out of looting.
            let _ = behavior.try_start(Action::Idle { timer: 0.5 });
            continue;
        }
        let corpse_dot = NpcDot { uid: target_uid };
        transfers.push((*looter_dot, corpse_dot));
    }

    for (looter_dot, corpse_dot) in transfers {
        // Pull one item from the corpse into the looter (if room).
        // Borrow each NPC sequentially to avoid double-mut.
        let item_taken = {
            let Some(corpse) = sim.0.npcs.get_mut(&corpse_dot.uid) else {
                continue;
            };
            // Pop in priority order: primary, secondary, helmet, armor,
            // relics, then general items.
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
        let Some(item) = item_taken else { continue };

        if let Some(looter) = sim.0.npcs.get_mut(&looter_dot.uid) {
            // Compute the looter's actual general-pouch capacity from
            // their rank and equipped armor's inventory_slots bonus.
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
