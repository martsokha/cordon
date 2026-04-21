//! Day-cycle systems: detect day rollovers and drive the per-day
//! world-event roll / expiry passes.

use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::GlobalRng;
use cordon_core::primitive::Day;
use cordon_data::gamedata::GameDataResource;

use super::events::DayRolled;
use super::world_events::{expire_events, roll_daily_events};
use crate::resources::{EventLog, FactionIndex, GameClock, PlayerIntel};

/// Track the previously-seen day so we can fire `DayRolled` exactly
/// once per in-game day rollover, no matter how many frames pass per
/// in-game minute.
#[derive(Default)]
pub(super) struct LastDay(pub Option<Day>);

pub(super) fn detect_day_rollover(
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

pub(super) fn roll_today_events(
    clock: Res<GameClock>,
    game_data: Res<GameDataResource>,
    factions: Res<FactionIndex>,
    mut events: ResMut<EventLog>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let event_defs: Vec<_> = game_data.0.events.values().cloned().collect();
    // Events don't care about spawn weights — strip them down to just
    // the ids for `roll_daily_events`.
    let faction_ids: Vec<_> = factions.0.iter().map(|(id, _)| id.clone()).collect();
    roll_daily_events(
        &mut events.0,
        &event_defs,
        &faction_ids,
        clock.0.day,
        &mut **rng,
    );
}

pub(super) fn expire_old_events(clock: Res<GameClock>, mut events: ResMut<EventLog>) {
    expire_events(&mut events.0, clock.0.day);
}

pub(super) fn expire_old_intel(
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut intel: ResMut<PlayerIntel>,
) {
    intel.expire(clock.0.day, &data.0.intel);
}
