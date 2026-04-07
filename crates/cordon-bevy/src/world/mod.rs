//! Simulation world wiring.
//!
//! Initializes the cordon-sim resources on entering the Playing state
//! and ticks the game clock. Population spawning lives in
//! [`cordon_sim::plugin::CordonSimPlugin`] which is added by
//! `main.rs`; every NPC and squad is a Bevy entity, not a HashMap
//! entry.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::player::PlayerState;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{AreaStates, EventLog, FactionIndex, GameClock, Player};
use cordon_sim::world::sectors::AreaState;

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

fn tick_game_time(time: Res<Time>, mut acc: ResMut<TimeAccumulator>, mut clock: ResMut<GameClock>) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}

/// Build the cordon-sim resource set from loaded game data and
/// insert each one.
pub fn init_world(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;

    let faction_ids = data.faction_ids();

    let mut areas: HashMap<_, _> = HashMap::new();
    for id in data.area_ids() {
        areas.insert(id.clone(), AreaState::new(id.clone()));
    }

    commands.insert_resource(GameClock::default());
    commands.insert_resource(Player(PlayerState::new(&faction_ids)));
    commands.insert_resource(FactionIndex(faction_ids));
    commands.insert_resource(AreaStates(areas));
    commands.insert_resource(EventLog::default());

    info!("World initialised; population will be spawned by cordon-sim");
}
