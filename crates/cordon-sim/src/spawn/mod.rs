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

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::entity::name::NamePool;
use cordon_core::entity::npc::Npc as NpcData;
use cordon_core::primitive::Uid;
use cordon_data::gamedata::GameDataResource;
use rand::{Rng, RngExt};

use crate::components::{NpcBundle, NpcId, SquadBundle, SquadMembership};
use crate::resources::{FactionIndex, GameClock, SquadIdIndex, UidAllocator};
use crate::spawn::generator::{
    DefaultNpcGenerator, LoadoutContext, NpcGenerator, roll_population_top_up,
};

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
    game_data: Res<GameDataResource>,
    mut squad_index: ResMut<SquadIdIndex>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    alive_npcs: Query<(), With<NpcId>>,
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

    // Pre-collect the area centres so we can pick a home position per
    // squad without re-borrowing rng.
    let area_centres: Vec<Vec2> = data
        .areas
        .values()
        .map(|a| Vec2::new(a.location.x, a.location.y))
        .collect();

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

        let home = pick_home_position(&area_centres, &mut **rng);

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

    // 06:00 = 0.25, 21:00 = 0.875.
    const DAY_START: f32 = 0.25;
    const DAY_END: f32 = 0.875;
    let lower = now_progress.clamp(DAY_START, DAY_END);
    for i in 1..n {
        let chunk = base + if i < extra { 1 } else { 0 };
        if chunk == 0 {
            continue;
        }
        let t = if lower >= DAY_END {
            (now_progress + 0.001 * i as f32).min(0.9999)
        } else {
            rng.random_range(lower..DAY_END)
        };
        waves.push((t, chunk));
    }

    waves.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    waves
}

/// Pick a random area centre with a small jitter, used as a squad's
/// initial spawn position.
fn pick_home_position<R: Rng>(centres: &[Vec2], rng: &mut R) -> Vec2 {
    if centres.is_empty() {
        return Vec2::ZERO;
    }
    let idx = rng.random_range(0..centres.len());
    let base = centres[idx];
    let jx = rng.random_range(-30.0_f32..30.0);
    let jy = rng.random_range(-30.0_f32..30.0);
    base + Vec2::new(jx, jy)
}
