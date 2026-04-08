//! World resource bootstrap + game clock tick.
//!
//! `init_world_resources` builds the cordon-sim resource set from
//! loaded [`GameDataResource`] (player state, area states, faction
//! weights, settlements, clock, event log). The cordon-bevy layer
//! calls it once on `OnEnter(AppState::Playing)`; every sim system
//! then runs gated on `resource_exists::<GameClock>`.
//!
//! `tick_game_time` is the per-frame clock advancer. It reads the
//! virtual `Time` delta so the cordon-bevy time-scale cheat
//! accelerates game-minutes alongside everything else that uses
//! `delta_secs()`.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::player::PlayerState;
use cordon_core::world::area::AreaKind;
use cordon_data::gamedata::GameDataResource;

use crate::resources::{
    AreaState, AreaStates, EventLog, FactionIndex, FactionSettlements, GameClock, Player,
};

/// How many game minutes pass per real second at 1× time scale.
/// A game day at this rate is 12 real minutes; the F4 debug cheat
/// in cordon-bevy multiplies this via `Time<Virtual>`.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

/// Per-frame fractional accumulator for [`tick_game_time`]. Keeps
/// sub-minute progress between frames so the clock doesn't
/// discretely jump whenever a whole minute happens to align with
/// a frame boundary.
#[derive(Resource, Default, Debug)]
pub struct TimeAccumulator(pub f32);

/// Build the cordon-sim resource set from loaded game data. The
/// caller is responsible for calling this exactly once, typically
/// on `OnEnter(PlayingState)` in the cordon-bevy layer.
pub fn init_world_resources(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;

    let faction_ids = data.faction_ids();
    // Pair each faction with its spawn weight from config so the
    // spawn system can do a weighted pick without re-reading the
    // FactionDef catalog every wave.
    let faction_weights: Vec<(_, u32)> = faction_ids
        .iter()
        .map(|id| {
            let weight = data.factions.get(id).map(|f| f.spawn_weight).unwrap_or(1);
            (id.clone(), weight)
        })
        .collect();

    let mut areas: HashMap<_, _> = HashMap::with_capacity(data.areas.len());
    for id in data.area_ids() {
        areas.insert(id.clone(), AreaState::new(id.clone()));
    }

    // Pre-collect each faction's Settlement centres so the spawn
    // system doesn't have to walk every area every wave. Built once
    // here because settlements are static config — they don't
    // change at runtime.
    let mut settlements: HashMap<_, Vec<Vec2>> = HashMap::with_capacity(faction_ids.len());
    for area in data.areas.values() {
        if let AreaKind::Settlement { faction, .. } = &area.kind {
            settlements
                .entry(faction.clone())
                .or_default()
                .push(Vec2::new(area.location.x, area.location.y));
        }
    }

    commands.insert_resource(GameClock::default());
    commands.insert_resource(Player(PlayerState::new(&faction_ids)));
    commands.insert_resource(FactionIndex(faction_weights));
    commands.insert_resource(FactionSettlements(settlements));
    commands.insert_resource(AreaStates(areas));
    commands.insert_resource(EventLog::default());
    commands.insert_resource(TimeAccumulator::default());

    info!("World initialised; population will be spawned by cordon-sim");
}

/// Per-frame clock advance. Reads `Res<Time>` (which is virtual
/// time by default in Bevy 0.18), so time-scale cheats applied via
/// `Time<Virtual>::set_relative_speed` naturally accelerate the
/// game clock along with the rest of the sim.
pub fn tick_game_time(
    time: Res<Time>,
    mut acc: ResMut<TimeAccumulator>,
    mut clock: ResMut<GameClock>,
) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}
