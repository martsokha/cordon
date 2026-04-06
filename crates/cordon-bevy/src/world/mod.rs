//! Simulation world as a Bevy resource.
//!
//! Creates the [`World`](cordon_sim::state::world::World) on entering
//! InGame state, runs the first morning phase to generate initial NPCs,
//! and provides the world state for other systems to read.

use bevy::prelude::*;
use cordon_core::entity::name::NamePool;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::simulation::day;
use cordon_sim::simulation::npcs::DefaultNpcGenerator;
use cordon_sim::state::world::World;

use crate::AppState;

pub struct WorldPlugin;

/// How many game minutes pass per real second.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeAccumulator(0.0));
        app.add_systems(OnEnter(AppState::Playing), init_world);
        app.add_systems(Update, tick_game_time.run_if(in_state(AppState::Playing)));
    }
}

#[derive(Resource)]
struct TimeAccumulator(f32);

fn tick_game_time(time: Res<Time>, mut acc: ResMut<TimeAccumulator>, mut sim: ResMut<SimWorld>) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        sim.0.time.advance_minutes(minutes);
    }
}

/// Bevy resource wrapping the simulation world.
#[derive(Resource)]
pub struct SimWorld(pub World);

pub fn init_world(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;

    let faction_ids = data.faction_ids();
    let area_ids = data.area_ids();

    let seed = rand::random::<u64>();
    let mut world = World::new(seed, faction_ids, &area_ids);

    let faction_pools = data.faction_name_pools();
    let fallback = NamePool {
        id: "fallback".into(),
        format: cordon_core::entity::name::NameFormat::Alias,
        names: vec![],
        surnames: vec![],
        aliases: vec!["alias-ghost".to_string()],
    };

    let npc_gen = DefaultNpcGenerator;
    let result = day::advance_day(
        &mut world,
        &data.events.values().cloned().collect::<Vec<_>>(),
        &npc_gen,
        &faction_pools,
        &fallback,
    );

    info!(
        "Day 1: {} visitors, {} events",
        result.visitors.len(),
        result.events_started
    );
    for npc in &result.visitors {
        let uid = npc.id;
        world.npcs.insert(uid, npc.clone());
    }

    commands.insert_resource(SimWorld(world));
}
