//! NPC and squad spawning as Bevy systems.
//!
//! [`spawn_population`] reads the live alive-NPC count from a Bevy
//! query, computes the deficit toward the generator's target
//! population, rolls fresh NPCs and squads via
//! [`generator::roll_population_top_up`], and spawns them as ECS
//! entities.
//!
//! Squad members reference each other via `Entity` so the AI hot path
//! never has to do a HashMap probe by Uid.

pub mod generator;
pub mod loadout;
pub mod relics;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NamePool;
use cordon_core::entity::npc::Npc as NpcData;
use cordon_core::primitive::{Id, Uid};
use cordon_data::gamedata::GameDataResource;
use rand::{Rng, RngExt};

use crate::components::{NpcBundle, NpcMarker, SquadBundle, SquadMembership};
use crate::resources::{FactionIndex, FactionSettlements, GameClock, SquadIdIndex, UidAllocator};
use crate::spawn::generator::{
    DefaultNpcGenerator, LoadoutContext, NpcGenerator, roll_population_top_up,
};
use crate::tuning::{SPAWN_DAY_END, SPAWN_DAY_START};

/// A fresh squad just entered the world. Currently declared
/// but not wired up — reserved as a seed for future spawn →
/// visual/audio hooks.
#[derive(Message, Debug, Clone)]
pub struct SquadSpawned {
    pub entity: Entity,
    pub faction: Id<Faction>,
}

/// Per-day spawn schedule: a list of `(day_progress, chunk_size)`
/// pairs picked at the start of each in-game day. Each entry fires
/// once when the in-game day progress reaches it; chunks are then
/// drained from the front. Population is replenished in a handful of
/// waves at randomized times of day rather than instantly on death.
#[derive(Default)]
pub struct SpawnSchedule {
    /// In-game day this schedule was generated for. `None` before the
    /// first plan is built.
    day: Option<u32>,
    /// Pending waves for the current day, sorted ascending by
    /// `day_progress`. Each tuple is `(day_progress_0_to_1, chunk_size)`.
    waves: Vec<(f32, u32)>,
}

/// Top up the alive NPC population in randomized chunks across the
/// in-game day. At each day rollover the system computes the current
/// deficit toward the generator's target, splits it into 3–5 waves,
/// and assigns each wave a random in-day timestamp (skewed toward
/// daytime hours). Waves fire one at a time as game time advances.
///
/// Also maintains the [`SquadIdIndex`] resource so AI systems can
/// resolve `Uid<Squad>` → `Entity` for goal references like
/// `Goal::Protect { other }`.
pub fn spawn_population(
    mut commands: Commands,
    mut schedule: Local<SpawnSchedule>,
    clock: Res<GameClock>,
    mut uids: ResMut<UidAllocator>,
    factions: Res<FactionIndex>,
    settlements: Res<FactionSettlements>,
    game_data: Res<GameDataResource>,
    mut squad_index: ResMut<SquadIdIndex>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    alive_npcs: Query<(), With<NpcMarker>>,
) {
    let data = &game_data.0;

    let generator = DefaultNpcGenerator;
    let day = clock.0.day.value();
    let day_progress = clock.0.day_progress();

    // Plan a fresh wave schedule on day rollover (or on first run).
    if schedule.day != Some(day) {
        let current = alive_npcs.iter().count() as u32;
        let target = generator.target_population(day);
        let deficit = target.saturating_sub(current);
        schedule.day = Some(day);
        schedule.waves = plan_daily_waves(deficit, day_progress, &mut **rng);
    }

    // Drain any waves whose scheduled in-day progress has been reached.
    let mut chunk: u32 = 0;
    while schedule
        .waves
        .first()
        .is_some_and(|(t, _)| *t <= day_progress)
    {
        chunk = chunk.saturating_add(schedule.waves.remove(0).1);
    }
    if chunk == 0 {
        return;
    }

    // Re-clamp the chunk against the live deficit so we never overshoot
    // the target if NPCs spawned between waves.
    let current = alive_npcs.iter().count() as u32;
    let target = generator.target_population(day);
    let deficit = target.saturating_sub(current);
    let deficit = chunk.min(deficit);
    if deficit == 0 {
        return;
    }

    let area_ids = data.area_ids();
    let faction_pools = data.faction_name_pools();
    let fallback = NamePool {
        id: "fallback".into(),
        format: cordon_core::entity::name::NameFormat::Alias,
        names: vec![],
        surnames: vec![],
        aliases: vec!["alias-ghost".to_string()],
    };
    let loadout_ctx = LoadoutContext {
        archetypes: &data.archetypes,
        items: &data.items,
        areas: &data.areas,
    };

    let spawn = roll_population_top_up(
        &mut **rng,
        &mut uids,
        &factions,
        &generator,
        &faction_pools,
        &fallback,
        &loadout_ctx,
        &area_ids,
        deficit,
    );

    // Pass 1: spawn NPC entities, mapping their uid → entity.
    let mut uid_to_entity: HashMap<Uid<NpcData>, Entity> = HashMap::with_capacity(spawn.npcs.len());
    for npc in spawn.npcs {
        let uid = npc.id;
        let entity = commands.spawn(NpcBundle::from_npc(npc)).id();
        uid_to_entity.insert(uid, entity);
    }

    // Settlement centres per faction come from the pre-built
    // [`FactionSettlements`] resource (built once at world init in
    // the bevy layer) so we don't walk all areas every spawn wave.
    let faction_settlements = &settlements.0;

    // Pass 2: spawn squads, resolving member uids → entities, and tag
    // each member with a SquadMembership component pointing back at
    // the squad entity.
    for squad in spawn.squads {
        let member_entities: Vec<Entity> = squad
            .members
            .iter()
            .filter_map(|uid| uid_to_entity.get(uid).copied())
            .collect();
        if member_entities.is_empty() {
            continue;
        }
        let leader_entity = match uid_to_entity.get(&squad.leader).copied() {
            Some(e) => e,
            None => member_entities[0],
        };

        let home = pick_faction_home(faction_settlements, &squad.faction, &mut **rng);

        let squad_uid = squad.id;
        let squad_entity = commands
            .spawn(SquadBundle::from_squad(
                squad,
                leader_entity,
                member_entities.clone(),
                home,
            ))
            .id();
        squad_index.0.insert(squad_uid, squad_entity);

        for (slot_idx, member_entity) in member_entities.iter().enumerate() {
            commands.entity(*member_entity).insert(SquadMembership {
                squad: squad_entity,
                slot: slot_idx as u8,
            });
        }
    }
}

/// Plan a fresh set of spawn waves for the current in-game day.
///
/// Splits `deficit` across 3–5 chunks and assigns each chunk a random
/// in-day timestamp in `[0.0, 1.0)`, skewed toward the 06:00–21:00
/// daytime window so the world feels alive when the player is active.
/// The first wave is forced to fire immediately at `now_progress` so
/// day rollovers replenish without a noticeable lag and so the very
/// first day seeds the initial population.
fn plan_daily_waves<R: Rng>(deficit: u32, now_progress: f32, rng: &mut R) -> Vec<(f32, u32)> {
    if deficit == 0 {
        return Vec::new();
    }
    let n_waves: u32 = rng.random_range(3..=5);
    let n = n_waves.min(deficit).max(1);

    let base = deficit / n;
    let extra = deficit % n;
    let mut waves: Vec<(f32, u32)> = Vec::with_capacity(n as usize);

    let first_size = base + if extra > 0 { 1 } else { 0 };
    waves.push((now_progress, first_size));

    let lower = now_progress.clamp(SPAWN_DAY_START, SPAWN_DAY_END);
    for i in 1..n {
        let chunk = base + if i < extra { 1 } else { 0 };
        if chunk == 0 {
            continue;
        }
        let t = if lower >= SPAWN_DAY_END {
            (now_progress + 0.001 * i as f32).min(0.9999)
        } else {
            rng.random_range(lower..SPAWN_DAY_END)
        };
        waves.push((t, chunk));
    }

    waves.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    waves
}

/// Pick a random Settlement centre belonging to `faction`, with a
/// small jitter, used as a squad's initial spawn position. If the
/// faction has no settlements (e.g., a faction defined in data but
/// with no held areas), fall back to the origin.
fn pick_faction_home<R: Rng>(
    settlements: &HashMap<Id<Faction>, Vec<Vec2>>,
    faction: &Id<Faction>,
    rng: &mut R,
) -> Vec2 {
    let Some(centres) = settlements.get(faction) else {
        return Vec2::ZERO;
    };
    if centres.is_empty() {
        return Vec2::ZERO;
    }
    let idx = rng.random_range(0..centres.len());
    let base = centres[idx];
    let jx = rng.random_range(-30.0_f32..30.0);
    let jy = rng.random_range(-30.0_f32..30.0);
    base + Vec2::new(jx, jy)
}
