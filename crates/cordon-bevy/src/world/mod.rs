//! Simulation world as a Bevy resource.
//!
//! Creates the [`World`](cordon_sim::state::world::World) on entering
//! InGame state, runs the first morning phase to generate initial NPCs,
//! and provides the world state for other systems to read.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::NamePool;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::simulation::day::PeriodResult;
use cordon_sim::simulation::npcs::DefaultNpcGenerator;
use cordon_sim::state::world::World;

use crate::AppState;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing), init_world);
    }
}

/// Bevy resource wrapping the simulation world.
#[derive(Resource)]
pub struct SimWorld(pub World);

/// Bevy resource holding the resolved faction→namepool mapping.
#[derive(Resource)]
pub struct FactionNamePools(pub HashMap<Id<Faction>, NamePool>);

/// Fallback name pool for factions with no configured pool.
#[derive(Resource)]
pub struct FallbackNamePool(pub NamePool);

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
    let result = cordon_sim::simulation::day::advance_period(
        &mut world,
        &data.events.values().cloned().collect::<Vec<_>>(),
        &npc_gen,
        &faction_pools,
        &fallback,
    );

    if let PeriodResult::Working {
        visitors,
        events_started,
    } = &result
    {
        info!(
            "Day 1: {} visitors, {} events",
            visitors.len(),
            events_started
        );
        for npc in visitors {
            let uid = npc.id;
            world.npcs.insert(uid, npc.clone());
        }
    }

    commands.insert_resource(SimWorld(world));
    commands.insert_resource(FactionNamePools(faction_pools));
    commands.insert_resource(FallbackNamePool(fallback));
}
