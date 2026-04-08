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
use cordon_core::world::area::AreaKind;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{
    AreaState, AreaStates, EventLog, FactionIndex, FactionSettlements, GameClock, Player,
};

use crate::AppState;

pub struct WorldPlugin;

/// How many game minutes pass per real second at 1× time scale.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

/// Player-selected time scale. Applied to `Time<Virtual>` so every
/// sim system that reads `delta_secs()` accelerates in lockstep
/// (combat, movement, goals, throttles, fire cooldowns). Real-time
/// systems that should *not* accelerate (UI smoothing, camera lerp)
/// must read `Res<Time<Real>>` explicitly.
#[derive(Resource, Debug, Clone, Copy)]
pub struct TimeAcceleration {
    pub multiplier: f32,
}

impl Default for TimeAcceleration {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

/// Time-scale presets cycled by the F4 debug key. 1× is real time,
/// 64× is "accelerated sim for skipping a few game hours".
const TIME_SCALE_PRESETS: &[f32] = &[1.0, 4.0, 16.0, 64.0];

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeAccumulator(0.0));
        app.insert_resource(TimeAcceleration::default());
        app.add_systems(OnEnter(AppState::Playing), init_world);
        app.add_systems(
            Update,
            (apply_time_scale, tick_game_time)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
        #[cfg(debug_assertions)]
        app.add_systems(Update, cheat_cycle_time_scale);
    }
}

#[derive(Resource)]
struct TimeAccumulator(f32);

/// Push [`TimeAcceleration.multiplier`] into Bevy's virtual time.
/// `Time<Virtual>::set_relative_speed` scales every subsequent
/// `delta_secs` read from `Res<Time>` (which aliases virtual time
/// by default), so sim systems naturally run faster without any
/// per-system changes.
fn apply_time_scale(accel: Res<TimeAcceleration>, mut virt: ResMut<Time<Virtual>>) {
    if !accel.is_changed() {
        return;
    }
    virt.set_relative_speed(accel.multiplier.max(0.0));
}

fn tick_game_time(time: Res<Time>, mut acc: ResMut<TimeAccumulator>, mut clock: ResMut<GameClock>) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}

/// F4 → cycle through [`TIME_SCALE_PRESETS`]. Dev cheat, compiled
/// out of release builds.
#[cfg(debug_assertions)]
fn cheat_cycle_time_scale(
    keys: Res<ButtonInput<KeyCode>>,
    mut accel: ResMut<TimeAcceleration>,
) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }
    let current = accel.multiplier;
    // Find the next preset strictly greater than the current; wrap
    // to the smallest if we're already at the top.
    let next = TIME_SCALE_PRESETS
        .iter()
        .copied()
        .find(|&s| s > current + 0.01)
        .unwrap_or(TIME_SCALE_PRESETS[0]);
    accel.multiplier = next;
    info!("cheat: time scale → {next}×");
}

/// Build the cordon-sim resource set from loaded game data and
/// insert each one.
pub fn init_world(mut commands: Commands, game_data: Res<GameDataResource>) {
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
    // here because settlements are static config — they don't change
    // at runtime.
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

    info!("World initialised; population will be spawned by cordon-sim");
}
