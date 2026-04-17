//! Radio broadcast delivery.
//!
//! Each frame, checks active events for radio entries whose delay
//! has elapsed. Grants intel on the first attempt regardless of
//! whether the player hears the broadcast. Emits a
//! [`RadioBroadcast`] message for the audio/UI layer.
//!
//! Missable broadcasts (`missable: true`, the default) fire once
//! and are marked delivered immediately. Non-missable broadcasts
//! keep emitting every frame until the audio layer confirms
//! receipt, or the event expires.

use bevy::prelude::*;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Event, Intel};
use cordon_data::gamedata::GameDataResource;

use crate::resources::{EventLog, GameClock, PlayerIntel};

/// Message emitted when a radio broadcast is due. The bunker
/// radio module consumes this to play audio. For non-missable
/// broadcasts, this fires every frame until [`BroadcastHeard`]
/// is written back.
#[derive(Message, Debug, Clone)]
pub struct RadioBroadcast {
    /// The event that triggered this broadcast.
    pub event: Id<Event>,
    /// Intel entries granted by this broadcast.
    pub intel: Vec<Id<Intel>>,
}

/// Written by the bunker radio module when it actually plays a
/// broadcast. The delivery system reads this to mark non-missable
/// broadcasts as delivered.
#[derive(Message, Debug, Clone)]
pub struct BroadcastHeard {
    pub event: Id<Event>,
    pub day_started: u32,
}

/// Tracks delivery state per active event instance.
#[derive(Resource, Default, Debug)]
pub struct DeliveredBroadcasts {
    entries: Vec<BroadcastState>,
}

#[derive(Debug, Clone)]
struct BroadcastState {
    def_id: Id<Event>,
    day_started: u32,
    /// Intel has been granted (happens once, regardless of audio).
    intel_granted: bool,
    /// Audio has been delivered (or was missable and skipped).
    delivered: bool,
}

impl DeliveredBroadcasts {
    fn find(&self, def_id: &Id<Event>, day_started: u32) -> Option<&BroadcastState> {
        self.entries
            .iter()
            .find(|e| &e.def_id == def_id && e.day_started == day_started)
    }

    fn find_mut(&mut self, def_id: &Id<Event>, day_started: u32) -> Option<&mut BroadcastState> {
        self.entries
            .iter_mut()
            .find(|e| &e.def_id == def_id && e.day_started == day_started)
    }

    fn get_or_insert(&mut self, def_id: Id<Event>, day_started: u32) -> &mut BroadcastState {
        if self.find(&def_id, day_started).is_none() {
            self.entries.push(BroadcastState {
                def_id: def_id.clone(),
                day_started,
                intel_granted: false,
                delivered: false,
            });
        }
        self.find_mut(&def_id, day_started).unwrap()
    }

    /// Prune entries for events that are no longer active.
    pub fn retain_active(&mut self, active_keys: &[(Id<Event>, u32)]) {
        self.entries.retain(|e| {
            active_keys
                .iter()
                .any(|(id, day)| id == &e.def_id && *day == e.day_started)
        });
    }
}

/// Per-frame: deliver radio broadcasts for active events.
pub fn deliver_radio_broadcasts(
    clock: Res<GameClock>,
    events: Res<EventLog>,
    data: Res<GameDataResource>,
    mut intel: ResMut<PlayerIntel>,
    mut delivered: ResMut<DeliveredBroadcasts>,
    mut broadcast_tx: MessageWriter<RadioBroadcast>,
) {
    let now = &clock.0;

    for active in &events.0 {
        let Some(def) = data.0.events.get(&active.def_id) else {
            continue;
        };
        let Some(radio) = &def.radio else {
            continue;
        };

        let state = delivered.get_or_insert(active.def_id.clone(), active.day_started.value());
        if state.delivered {
            continue;
        }

        // Minutes elapsed since the event started.
        let event_start_minutes = (active.day_started.value() - 1) as u64 * 24 * 60;
        let elapsed = now.total_minutes().saturating_sub(event_start_minutes) as u32;

        if elapsed < radio.delay_minutes {
            continue;
        }

        // Grant intel once, regardless of whether audio plays.
        if !state.intel_granted {
            for intel_id in &radio.grants_intel {
                intel.grant(intel_id.clone(), now.day);
            }
            state.intel_granted = true;
        }

        // Missable: mark delivered immediately even if radio is off.
        // Non-missable: keep emitting until BroadcastHeard arrives.
        if radio.missable {
            state.delivered = true;
        }

        broadcast_tx.write(RadioBroadcast {
            event: active.def_id.clone(),
            intel: radio.grants_intel.clone(),
        });
    }
}

/// Mark non-missable broadcasts as delivered when the bunker
/// radio confirms playback.
pub fn process_broadcast_heard(
    mut delivered: ResMut<DeliveredBroadcasts>,
    mut heard: MessageReader<BroadcastHeard>,
) {
    for msg in heard.read() {
        if let Some(state) = delivered.find_mut(&msg.event, msg.day_started) {
            state.delivered = true;
        }
    }
}

/// Prune delivered-broadcast tracking for expired events.
pub fn prune_delivered_broadcasts(
    events: Res<EventLog>,
    mut delivered: ResMut<DeliveredBroadcasts>,
) {
    let active_keys: Vec<_> = events
        .0
        .iter()
        .map(|e| (e.def_id.clone(), e.day_started.value()))
        .collect();
    delivered.retain_active(&active_keys);
}
