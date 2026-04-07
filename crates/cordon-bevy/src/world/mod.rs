//! Simulation world wiring.
//!
//! Creates the [`SimWorld`] resource on entering the Playing state and
//! ticks game time. Population spawning lives in the
//! [`cordon_sim::plugin::CordonSimPlugin`] which is added by `main.rs`;
//! every NPC and squad is a Bevy entity, not a HashMap entry.

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::world::day;
use cordon_sim::world::state::World;

use crate::AppState;

pub use cordon_sim::resources::SimWorld;

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

fn tick_game_time(
    time: Res<Time>,
    mut acc: ResMut<TimeAccumulator>,
    mut sim: ResMut<SimWorld>,
) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        sim.0.time.advance_minutes(minutes);
    }
}

pub fn init_world(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;
    let faction_ids = data.faction_ids();
    let area_ids = data.area_ids();

    let seed = rand::random::<u64>();
    let mut world = World::new(seed, faction_ids, &area_ids);

    // Run the day-tick logic once for events; population spawn now
    // happens in the cordon-sim spawn system, automatically each frame.
    let event_defs: Vec<_> = data.events.values().cloned().collect();
    let _ = day::advance_day(&mut world, &event_defs);

    info!("World initialised; population will be spawned by cordon-sim");
    commands.insert_resource(SimWorld(world));
}
