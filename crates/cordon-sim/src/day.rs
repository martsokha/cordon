//! Day rollover detection and the per-day systems.
//!
//! [`detect_day_rollover`] watches the [`GameClock`] each frame and
//! writes a [`DayRolled`] message whenever the day number advances.
//! Per-day work — daily event rolls, faction reactions, event
//! expiry — runs as separate systems gated on the message.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::primitive::Day;
use cordon_data::gamedata::GameDataResource;

use crate::events::DayRolled;
use crate::plugin::SimSet;
use crate::resources::{AreaStates, EventLog, FactionIndex, GameClock, Player};
use crate::world::events::{expire_events, roll_daily_events};
use crate::world::factions::tick_factions;

pub struct DayCyclePlugin;

impl Plugin for DayCyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DayRolled>();
        app.add_systems(
            Update,
            (
                detect_day_rollover,
                roll_today_events.run_if(on_message::<DayRolled>),
                react_to_events.run_if(on_message::<DayRolled>),
                expire_old_events.run_if(on_message::<DayRolled>),
            )
                .chain()
                .in_set(SimSet::Cleanup),
        );
    }
}

/// Track the previously-seen day so we can fire `DayRolled` exactly
/// once per in-game day rollover, no matter how many frames pass per
/// in-game minute.
#[derive(Default)]
struct LastDay(Option<Day>);

fn detect_day_rollover(
    clock: Res<GameClock>,
    mut last: Local<LastDay>,
    mut rolled: MessageWriter<DayRolled>,
) {
    let today = clock.0.day;
    if last.0 != Some(today) {
        if last.0.is_some() {
            // Genuine rollover (not the very first frame).
            rolled.write(DayRolled { new_day: today });
        } else {
            // First time we see the clock — emit so day-1 systems run too.
            rolled.write(DayRolled { new_day: today });
        }
        last.0 = Some(today);
    }
}

fn roll_today_events(
    clock: Res<GameClock>,
    game_data: Res<GameDataResource>,
    factions: Res<FactionIndex>,
    mut events: ResMut<EventLog>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let event_defs: Vec<_> = game_data.0.events.values().cloned().collect();
    roll_daily_events(
        &mut events.0,
        &event_defs,
        &factions.0,
        clock.0.day,
        &mut **rng,
    );
}

fn react_to_events(
    events: Res<EventLog>,
    mut areas: ResMut<AreaStates>,
    mut player: ResMut<Player>,
) {
    tick_factions(&events.0, &mut areas.0, &mut player.0);
}

fn expire_old_events(clock: Res<GameClock>, mut events: ResMut<EventLog>) {
    expire_events(&mut events.0, clock.0.day);
}
