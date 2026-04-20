//! Radio broadcast queue + dialogue playback.
//!
//! Sim emits [`RadioBroadcast`] when a broadcast's delay elapses.
//! We don't play it immediately — the player decides when to tune
//! in. The queue holds pending broadcasts until:
//!
//! - The player turns the radio on and no other dialogue is
//!   running → pop the oldest and start its yarn node.
//! - A broadcast's yarn dialogue completes → grant the broadcast's
//!   intel, then pop the next queued entry (if any).
//! - Day rolls over → drop missable entries from before today.
//!
//! Intel is granted only when the yarn node completes. Just
//! hearing the chatter audio or seeing the toast doesn't suffice
//! — the player has to actually listen through.

use bevy::prelude::*;
use bevy_yarnspinner::events::DialogueCompleted;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Event, Intel};
use cordon_sim::day::DayRolled;
use cordon_sim::day::radio::{BroadcastHeard, RadioBroadcast};
use cordon_sim::quest::messages::IntelGranted;
use cordon_sim::resources::{GameClock, PlayerIntel};

use super::systems::{RadioMarker, RadioOn};
use crate::bunker::resources::{CurrentDialogue, StartDialogue};
use crate::bunker::visitor::VisitorState;

/// One pending broadcast waiting for the player to tune in.
#[derive(Debug, Clone)]
pub struct QueuedBroadcast {
    pub event: Id<Event>,
    pub intel: Option<Id<Intel>>,
    pub yarn_node: String,
    pub arrived_day: u32,
    pub missable: bool,
}

/// FIFO queue of broadcasts waiting to be listened to.
///
/// Append on [`RadioBroadcast`], pop on playback. Missable entries
/// older than the current day are pruned at day rollover.
#[derive(Resource, Default, Debug)]
pub struct RadioQueue {
    entries: Vec<QueuedBroadcast>,
}

impl RadioQueue {
    pub fn push(&mut self, broadcast: QueuedBroadcast) {
        self.entries.push(broadcast);
    }

    pub fn pop_next(&mut self) -> Option<QueuedBroadcast> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entries.remove(0))
        }
    }
}

/// Tracks the broadcast currently being played through yarn. Set
/// when we start a broadcast's dialogue; cleared on
/// [`DialogueCompleted`] after intel is granted. Used to
/// distinguish "a broadcast dialogue just ended" from "a visitor
/// dialogue just ended" at the same [`DialogueCompleted`] fire.
#[derive(Resource, Default, Debug)]
pub struct ActiveBroadcast(pub Option<QueuedBroadcast>);

/// Ingest radio broadcasts emitted by the sim: push into the queue
/// and immediately confirm receipt so non-missable broadcasts stop
/// re-emitting.
pub(super) fn on_radio_broadcast(
    clock: Res<GameClock>,
    mut broadcasts: MessageReader<RadioBroadcast>,
    mut queue: ResMut<RadioQueue>,
    mut heard_tx: MessageWriter<BroadcastHeard>,
) {
    let day = clock.0.day.value();
    for msg in broadcasts.read() {
        queue.push(QueuedBroadcast {
            event: msg.event.clone(),
            intel: msg.intel.clone(),
            yarn_node: msg.yarn_node.clone(),
            arrived_day: day,
            missable: msg.missable,
        });
        heard_tx.write(BroadcastHeard {
            event: msg.event.clone(),
            day_started: day,
        });
    }
}

/// Try to start a queued broadcast's dialogue.
///
/// Fires when: radio is on, no visitor is around (knocking,
/// inside, or step-away waiting), the dialog panel is idle, no
/// broadcast is mid-playback, and the queue is non-empty. Runs
/// every frame because the gating conditions can flip in any
/// order (radio toggle, visitor dismiss, prior broadcast
/// completion).
///
/// The visitor gate covers the "someone's knocking" and "dialogue
/// is paused via step-away" cases that `CurrentDialogue::Idle`
/// alone wouldn't catch — a broadcast shouldn't interrupt those.
pub(super) fn try_start_next_broadcast(
    mut queue: ResMut<RadioQueue>,
    mut active: ResMut<ActiveBroadcast>,
    current_dialogue: Res<CurrentDialogue>,
    visitor_state: Res<VisitorState>,
    radio_q: Query<&RadioOn, With<RadioMarker>>,
    mut start_dialogue: MessageWriter<StartDialogue>,
) {
    if active.0.is_some() {
        return;
    }
    if !matches!(*current_dialogue, CurrentDialogue::Idle) {
        return;
    }
    if !matches!(*visitor_state, VisitorState::Quiet) {
        return;
    }
    let Ok(radio_on) = radio_q.single() else {
        return;
    };
    if !radio_on.is_on() {
        return;
    }
    let Some(next) = queue.pop_next() else {
        return;
    };
    let node = next.yarn_node.clone();
    info!("radio: playing broadcast `{}`", next.event.as_str());
    active.0 = Some(next);
    start_dialogue.write(StartDialogue { node });
}

/// Observer fired when any dialogue ends. If the dialogue belonged
/// to a broadcast (tracked via [`ActiveBroadcast`]), grant its
/// intel and clear the active slot so the next broadcast can start.
pub(super) fn on_broadcast_dialogue_completed(
    _event: On<DialogueCompleted>,
    clock: Res<GameClock>,
    mut active: ResMut<ActiveBroadcast>,
    mut intel: ResMut<PlayerIntel>,
    mut granted_tx: MessageWriter<IntelGranted>,
) {
    let Some(broadcast) = active.0.take() else {
        return;
    };
    let day = clock.0.day;
    if let Some(intel_id) = &broadcast.intel {
        // Grant unconditionally — duplicates are no-op'd by
        // `PlayerIntel::grant`, but we only announce when this is
        // actually new so the player doesn't see spam on a
        // re-listen of content they already have.
        let already_had = intel.has(intel_id);
        intel.grant(intel_id.clone(), day);
        if already_had {
            info!(
                "radio: broadcast `{}` read; intel `{}` already known",
                broadcast.event.as_str(),
                intel_id.as_str()
            );
        } else {
            info!(
                "radio: broadcast `{}` read; granted intel `{}`",
                broadcast.event.as_str(),
                intel_id.as_str()
            );
            granted_tx.write(IntelGranted {
                intel: intel_id.clone(),
            });
        }
    } else {
        info!(
            "radio: broadcast `{}` read (no intel grant)",
            broadcast.event.as_str()
        );
    }
}

/// Day rollover: drop missable broadcasts that arrived before
/// today. Non-missable entries stay in the queue indefinitely.
pub(super) fn prune_missable_on_day_roll(
    mut rolled: MessageReader<DayRolled>,
    clock: Res<GameClock>,
    mut queue: ResMut<RadioQueue>,
) {
    if rolled.read().next().is_none() {
        return;
    }
    let today = clock.0.day.value();
    queue
        .entries
        .retain(|b| !b.missable || b.arrived_day >= today);
}
