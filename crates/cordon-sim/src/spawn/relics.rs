//! Relic spawning and pickup.
//!
//! **Spawning**: triggered once per in-game day by the [`DayRolled`]
//! message. For each area with a hazard, the system counts live
//! relics anchored to that area and tops up toward a capacity
//! determined by the anomaly's intensity tier. Candidate relic defs
//! are filtered by [`RelicData::origin`] matching the area's hazard
//! kind, then weighted by [`Rarity`].
//!
//! **Pickup**: squads on a [`Goal::Scavenge`] walking near a relic
//! transfer the [`ItemInstance`] into the squad leader's loadout if
//! there's room in the relic slots, then despawn the relic entity
//! and emit [`RelicPickedUp`].
//!
//! Relic entities carry [`RelicMarker`], [`RelicItem`], [`RelicHome`]
//! (the anchoring area), and a [`Transform`] placed at a random
//! point inside the anomaly disk.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::item::{Item, ItemData, ItemDef, ItemInstance, RelicData};
use cordon_core::primitive::{HazardType, Id, Tier};
use cordon_core::world::area::AreaDef;
use cordon_data::gamedata::GameDataResource;
use rand::{Rng, RngExt};

use crate::behavior::Dead;
use crate::components::{
    LoadoutComp, NpcMarker, RelicHome, RelicItem, RelicMarker, SquadGoal, SquadLeader,
};
use crate::events::{DayRolled, RelicPickedUp};
use crate::plugin::SimSet;
use crate::tuning::{RELIC_ATTEMPTS_PER_AREA, RELIC_PICKUP_REACH, RELIC_SPAWN_PROBABILITY};

pub struct RelicSpawnPlugin;

impl Plugin for RelicSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RelicPickedUp>();
        app.add_systems(
            Update,
            (
                spawn_relics_on_day_rollover
                    .in_set(SimSet::Spawn)
                    .run_if(on_message::<DayRolled>),
                pickup_relics.in_set(SimSet::Loot),
            ),
        );
    }
}

/// Per-area relic cap, indexed by hazard intensity tier.
fn cap_for_intensity(intensity: Tier) -> u32 {
    match intensity {
        Tier::VeryLow => 1,
        Tier::Low => 2,
        Tier::Medium => 3,
        Tier::High => 4,
        Tier::VeryHigh => 5,
    }
}

fn spawn_relics_on_day_rollover(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    existing: Query<&RelicHome, With<RelicMarker>>,
) {
    let data = &game_data.0;
    let rng: &mut WyRand = &mut rng;

    // Count current relics per area so we can cap the top-up.
    let mut counts: HashMap<Id<cordon_core::world::area::Area>, u32> = HashMap::new();
    for home in existing.iter() {
        *counts.entry(home.0.clone()).or_insert(0) += 1;
    }

    // Index relic defs by origin hazard. Each entry is the def id,
    // the rolled weight contribution, and the full def.
    let mut by_origin: HashMap<HazardType, Vec<(Id<Item>, u32, &ItemDef, &RelicData)>> =
        HashMap::new();
    for (id, def) in &data.items {
        let ItemData::Relic(relic) = &def.data else {
            continue;
        };
        by_origin.entry(relic.origin).or_default().push((
            id.clone(),
            def.rarity.weight(),
            def,
            relic,
        ));
    }

    for area in data.areas.values() {
        let Some(hazard) = area.danger.hazard else {
            continue;
        };
        let cap = cap_for_intensity(hazard.intensity);
        let current = counts.get(&area.id).copied().unwrap_or(0);
        if current >= cap {
            continue;
        }

        let Some(candidates) = by_origin.get(&hazard.kind) else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }

        let mut remaining = cap - current;
        for _ in 0..RELIC_ATTEMPTS_PER_AREA {
            if remaining == 0 {
                break;
            }
            if rng.random::<f32>() >= RELIC_SPAWN_PROBABILITY {
                continue;
            }
            let Some((_id, _w, def, _relic)) = pick_weighted(candidates, rng) else {
                continue;
            };
            spawn_one(&mut commands, area, def, rng);
            remaining -= 1;
        }
    }
}

fn pick_weighted<'a, R: Rng>(
    candidates: &'a [(Id<Item>, u32, &'a ItemDef, &'a RelicData)],
    rng: &mut R,
) -> Option<&'a (Id<Item>, u32, &'a ItemDef, &'a RelicData)> {
    let total: u32 = candidates.iter().map(|(_, w, _, _)| *w).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.random_range(0..total);
    for entry in candidates {
        if roll < entry.1 {
            return Some(entry);
        }
        roll -= entry.1;
    }
    candidates.last()
}

fn spawn_one(commands: &mut Commands, area: &AreaDef, def: &ItemDef, rng: &mut WyRand) {
    // Random point inside the anomaly disk. Sqrt gives uniform area
    // distribution rather than center-biased.
    let r = area.radius.value() * rng.random::<f32>().sqrt();
    let theta = rng.random::<f32>() * std::f32::consts::TAU;
    let x = area.location.x + r * theta.cos();
    let y = area.location.y + r * theta.sin();

    let instance = ItemInstance::new(def);

    commands.spawn((
        RelicMarker,
        RelicItem(instance),
        RelicHome(area.id.clone()),
        Transform::from_xyz(x, y, 0.4),
    ));
}

/// Transfer nearby relics to scavenging squad leaders' loadouts.
///
/// Runs in [`SimSet::Loot`] so it sits alongside corpse looting. Only
/// squads with an active [`Goal::Scavenge`] goal collect relics, and
/// only if the relic is anchored to the squad's target scavenge area
/// — scavenging the railyard does not absorb relics from the factory.
fn pickup_relics(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut picked: MessageWriter<RelicPickedUp>,
    relics: Query<(Entity, &RelicItem, &RelicHome, &Transform), With<RelicMarker>>,
    squads: Query<(&SquadGoal, &SquadLeader)>,
    leader_positions: Query<&Transform, (With<NpcMarker>, Without<Dead>)>,
    mut leader_loadouts: Query<&mut LoadoutComp, (With<NpcMarker>, Without<Dead>)>,
) {
    use cordon_core::entity::squad::Goal;
    use cordon_core::item::{ArmorData, ItemData, Loadout};

    if relics.is_empty() {
        return;
    }
    let items = &game_data.0.items;

    // Snapshot every scavenging squad's leader entity, target area,
    // and current leader position.
    let mut scavengers: Vec<(
        Entity,
        cordon_core::primitive::Id<cordon_core::world::area::Area>,
        Vec2,
    )> = Vec::new();
    for (goal, leader) in squads.iter() {
        let Goal::Scavenge { area } = &goal.0 else {
            continue;
        };
        let Ok(leader_t) = leader_positions.get(leader.0) else {
            continue;
        };
        scavengers.push((leader.0, area.clone(), leader_t.translation.truncate()));
    }
    if scavengers.is_empty() {
        return;
    }

    let reach_sq = RELIC_PICKUP_REACH * RELIC_PICKUP_REACH;

    for (relic_entity, relic_item, home, relic_transform) in relics.iter() {
        let relic_pos = relic_transform.translation.truncate();
        // Pick the *closest* scavenging leader whose target area
        // matches this relic's home and who is within pickup reach.
        // First-match-wins would give the relic to whichever squad
        // happens to have the lower entity id, which is arbitrary
        // and can cause a further squad to "steal" the pickup from a
        // closer one.
        let hit = scavengers
            .iter()
            .filter(|(_, area, leader_pos)| {
                *area == home.0 && leader_pos.distance_squared(relic_pos) <= reach_sq
            })
            .min_by(|(_, _, a), (_, _, b)| {
                a.distance_squared(relic_pos)
                    .partial_cmp(&b.distance_squared(relic_pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(leader, _, _)| *leader);
        let Some(leader_entity) = hit else {
            continue;
        };

        let Ok(mut loadout_comp) = leader_loadouts.get_mut(leader_entity) else {
            continue;
        };

        let armor_def: Option<&ArmorData> = loadout_comp
            .0
            .armor
            .as_ref()
            .and_then(|inst| items.get(&inst.def_id))
            .and_then(|def| match &def.data {
                ItemData::Armor(a) => Some(a),
                _ => None,
            });
        let cap = Loadout::relic_capacity(armor_def);

        let item_id = relic_item.0.def_id.clone();
        if loadout_comp.0.add_relic(relic_item.0.clone(), cap).is_err() {
            // No room — leave the relic in the world.
            continue;
        }

        commands.entity(relic_entity).despawn();
        picked.write(RelicPickedUp {
            picker: leader_entity,
            item: item_id,
        });
    }
}
