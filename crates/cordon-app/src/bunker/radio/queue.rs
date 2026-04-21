//! Listening-mode state machine + broadcast queue.
//!
//! The player interacts with the radio → [`EnterListening`] fires
//! → [`handle_enter_listening`] locks movement + interaction, spawns
//! the static loop, and starts either the next queued broadcast or
//! the `radio_idle` yarn (single-line placeholder when nothing's
//! queued). Between broadcasts a 1.2s static-burst gap plays
//! before the next dialog opens. ESC or the `<<close_radio>>` yarn
//! command exits.
//!
//! Broadcasts arrive in the queue via the sim's [`RadioBroadcast`]
//! message (unchanged). Intel grants on the radio-broadcast
//! dialogue completing — if the player exits mid-broadcast, intel
//! stays ungranted and the broadcast stays queued. Per design the
//! exit affordance is only surfaced between broadcasts, so
//! "mid-broadcast exit" in practice only happens if the player
//! alt-tabs or the state is forced out by another system.

use std::time::Duration;

use bevy::prelude::*;
use bevy_yarnspinner::events::DialogueCompleted;
use cordon_core::primitive::Id;
use cordon_core::world::narrative::{Event, Intel};
use cordon_sim::day::DayRolled;
use cordon_sim::day::radio::{BroadcastHeard, RadioBroadcast};
use cordon_sim::quest::messages::IntelGranted;
use cordon_sim::resources::{GameClock, PlayerIntel};

use super::systems::{
    RadioAudio, RadioChatter, RadioMarker, RadioSfx, STATIC_BURST_VOLUME, TOGGLE_VOLUME,
    spawn_chatter, spawn_click, spawn_oneshot_at, spawn_static_loop,
};
use crate::bunker::resources::{
    CameraMode, CurrentDialogue, CurrentDialogueOwner, DialogueOwner, InteractionLocked,
    MovementLocked, StartDialogue, StopDialogue,
};
use crate::bunker::visitor::VisitorState;

/// Duration of the static burst played between back-to-back
/// broadcasts. Long enough to hear the radio audibly switch,
/// short enough to not feel like a load screen.
const STATIC_GAP_SECS: f32 = 1.2;

/// Yarn node played when the player is listening and no broadcast
/// is queued. Single line plus a `<<close_radio>>` option so the
/// player can back out.
const STATIC_YARN_NODE: &str = "broadcast_static";

/// Message: player requested to enter listening mode. Written by
/// the interact handler in [`systems`](super::systems); processed
/// by [`handle_enter_listening`].
#[derive(Message, Debug, Clone, Copy)]
pub(super) struct EnterListening;

/// Message: player requested to exit listening mode. Written by
/// ESC handling or the `<<close_radio>>` yarn command; processed
/// by [`handle_exit_listening`].
#[derive(Message, Debug, Clone, Copy)]
pub struct ExitListening;

/// State of the player's radio-listening session. `active` flips
/// true when `handle_enter_listening` runs and false the moment
/// the player commits to exiting (ESC, `<<close_radio>>`). Held
/// as a resource so any system can check whether listening is
/// currently happening without needing to wait on deferred
/// command application — the synchronous flip lets observers
/// gated on "are we listening?" see the correct answer in the
/// same frame the exit is requested.
#[derive(Resource, Default, Debug)]
pub struct ListeningToRadio {
    pub active: bool,
}

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

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The broadcast currently being read through the dialogue UI.
/// Cleared on [`DialogueCompleted`] after intel is granted; also
/// cleared without a grant on forced exit.
#[derive(Resource, Default, Debug)]
pub struct ActiveBroadcast(pub Option<QueuedBroadcast>);

/// Timer that runs during the gap between back-to-back broadcasts.
/// While active, the dialog UI is idle and the static burst is
/// audible; on timeout, the next broadcast starts.
#[derive(Resource, Debug)]
pub(super) struct StaticGapTimer(Timer);

/// Ingest radio broadcasts emitted by the sim: push into the queue
/// and immediately confirm receipt so non-missable broadcasts stop
/// re-emitting. No toast here — toasts fire on intel grant, not
/// on queue delivery.
///
/// If the player is currently listening and sitting on the idle
/// placeholder (no active broadcast, no static-gap timer), break
/// out of the idle yarn and let the dialog-completed observer
/// pick up the new broadcast. This makes "listen, nothing on, a
/// broadcast arrives" automatically transition to the fresh
/// broadcast instead of stranding the player on the idle screen.
pub(super) fn on_radio_broadcast(
    clock: Res<GameClock>,
    mut broadcasts: MessageReader<RadioBroadcast>,
    mut queue: ResMut<RadioQueue>,
    mut heard_tx: MessageWriter<BroadcastHeard>,
    listening: Res<ListeningToRadio>,
    active: Res<ActiveBroadcast>,
    gap_timer: Option<Res<StaticGapTimer>>,
    mut stop_dialogue: MessageWriter<StopDialogue>,
) {
    let day = clock.0.day.value();
    let mut had_new = false;
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
        had_new = true;
    }
    if had_new && listening.active && active.0.is_none() && gap_timer.is_none() {
        // Player is parked on the idle yarn; yank them out so the
        // dialog-completed observer handles the rest.
        stop_dialogue.write(StopDialogue);
    }
}

/// Transition into listening mode: lock player, spawn static loop,
/// start the first broadcast (or the idle yarn if nothing's
/// queued).
pub(super) fn handle_enter_listening(
    mut commands: Commands,
    mut events: MessageReader<EnterListening>,
    mut listening: ResMut<ListeningToRadio>,
    mut camera_mode: ResMut<CameraMode>,
    visitor_state: Res<VisitorState>,
    current_dialogue: Res<CurrentDialogue>,
    mut queue: ResMut<RadioQueue>,
    mut active: ResMut<ActiveBroadcast>,
    sfx: Res<RadioSfx>,
    radio_q: Query<(Entity, &GlobalTransform), With<RadioMarker>>,
    mut start_dialogue: MessageWriter<StartDialogue>,
) {
    if events.read().last().is_none() {
        return;
    }
    if listening.active {
        return;
    }
    if !matches!(*visitor_state, VisitorState::Quiet) {
        return;
    }
    if !matches!(*current_dialogue, CurrentDialogue::Idle) {
        return;
    }
    let Ok((radio_entity, radio_transform)) = radio_q.single() else {
        return;
    };
    let radio_pos = radio_transform.translation();

    info!("radio: entering listening mode");
    listening.active = true;
    commands.insert_resource(MovementLocked);
    commands.insert_resource(InteractionLocked);
    // Look at the radio prop — mirrors the visitor admit flow
    // where the camera slerps to face whoever's talking.
    *camera_mode = CameraMode::LookingAt { target: radio_pos };
    // Click + ambient static. Click is untagged so the exit
    // teardown doesn't cut it mid-play.
    spawn_click(&mut commands, &sfx.enable, TOGGLE_VOLUME, radio_pos);
    spawn_static_loop(&mut commands, radio_entity, &sfx);

    // Pick what to open with: the next queued broadcast, or the
    // idle placeholder node. When we open on a broadcast, start
    // the chatter sting too so the "tuned into a voice" layer
    // lands over the ambient static.
    let node = if let Some(next) = queue.pop_next() {
        let node = next.yarn_node.clone();
        active.0 = Some(next);
        spawn_chatter(&mut commands, radio_entity, &sfx);
        node
    } else {
        STATIC_YARN_NODE.to_string()
    };
    start_dialogue.write(StartDialogue {
        node,
        by: DialogueOwner::Radio,
    });
}

/// Exit listening mode: stop any static audio, remove locks,
/// clear the active broadcast without granting intel (the player
/// only exits between broadcasts per design, so no intel is
/// actually in flight), and stop the dialogue runner.
pub(super) fn handle_exit_listening(
    mut commands: Commands,
    mut events: MessageReader<ExitListening>,
    mut listening: ResMut<ListeningToRadio>,
    mut torn_down: Local<bool>,
    mut camera_mode: ResMut<CameraMode>,
    mut active: ResMut<ActiveBroadcast>,
    mut queue: ResMut<RadioQueue>,
    sfx: Res<RadioSfx>,
    radio_q: Query<&GlobalTransform, With<RadioMarker>>,
    radio_audio: Query<Entity, With<RadioAudio>>,
    mut stop_dialogue: MessageWriter<StopDialogue>,
) {
    // Reset the "already torn down" latch whenever listening is
    // active again — i.e., a fresh `handle_enter_listening` has
    // flipped `active` to true since the last exit.
    if listening.active {
        *torn_down = false;
    }
    if events.read().last().is_none() {
        return;
    }
    // Guard against a second `ExitListening` between the first
    // exit and the next `EnterListening`. `listening.active` is
    // synchronously flipped off by ESC / `<<close_radio>>`, so
    // reading it here can't distinguish "already torn down" from
    // "just requested." A local latch tracks that explicitly.
    if *torn_down {
        return;
    }
    *torn_down = true;

    info!("radio: exiting listening mode");
    // If the player bailed while a broadcast was mid-read, put it
    // back on the front of the queue so they can pick up where
    // they left off on re-entry. Shouldn't happen in normal
    // gameplay — exits are only exposed between broadcasts — but
    // handle it defensively.
    if let Some(unfinished) = active.0.take() {
        queue.entries.insert(0, unfinished);
    }

    // Disable click at the radio's spatial position, played before
    // the audio teardown so the click itself survives (untagged).
    if let Ok(transform) = radio_q.single() {
        spawn_click(
            &mut commands,
            &sfx.disable,
            TOGGLE_VOLUME,
            transform.translation(),
        );
    }

    listening.active = false;
    // Release the camera lock — matches visitor dismiss flow.
    if matches!(*camera_mode, CameraMode::LookingAt { .. }) {
        *camera_mode = CameraMode::Free;
    }
    commands.remove_resource::<MovementLocked>();
    commands.remove_resource::<InteractionLocked>();
    commands.remove_resource::<StaticGapTimer>();
    for entity in &radio_audio {
        commands.entity(entity).despawn();
    }
    stop_dialogue.write(StopDialogue);
}

/// Observer on [`DialogueCompleted`]: grant intel for the
/// just-finished broadcast (if any), then schedule what comes
/// next — a static-gap timer before the next queued broadcast,
/// or the idle yarn if the queue is empty.
pub(super) fn on_broadcast_dialogue_completed(
    _event: On<DialogueCompleted>,
    mut commands: Commands,
    owner: Res<CurrentDialogueOwner>,
    listening: Res<ListeningToRadio>,
    clock: Res<GameClock>,
    queue: Res<RadioQueue>,
    mut active: ResMut<ActiveBroadcast>,
    mut intel: ResMut<PlayerIntel>,
    mut granted_tx: MessageWriter<IntelGranted>,
    sfx: Res<RadioSfx>,
    radio_q: Query<&GlobalTransform, With<RadioMarker>>,
    chatter_q: Query<Entity, With<RadioChatter>>,
    mut start_dialogue: MessageWriter<StartDialogue>,
) {
    // Only act on dialog completions that belonged to the radio.
    // Visitor and quest dialog endings use their own owner tags.
    if !matches!(owner.0, DialogueOwner::Radio) {
        return;
    }
    // Second gate: the player may have ended listening already
    // this frame (ESC / `<<close_radio>>` both flip `active`
    // synchronously). The owner tag is still `Radio` — this dialog
    // was ours — but the session is closing, so we must not
    // restart the idle yarn or dip into the static gap.
    if !listening.active {
        return;
    }

    // Kill the chatter layer so the static loop is the only
    // thing heard during the gap / on the idle yarn.
    for entity in &chatter_q {
        commands.entity(entity).despawn();
    }

    // Grant intel for the just-finished broadcast, if any.
    if let Some(broadcast) = active.0.take() {
        let day = clock.0.day;
        if let Some(intel_id) = &broadcast.intel {
            let already_had = intel.has(intel_id);
            intel.grant(intel_id.clone(), day);
            if !already_had {
                granted_tx.write(IntelGranted {
                    intel: intel_id.clone(),
                });
                info!(
                    "radio: broadcast `{}` read; granted intel `{}`",
                    broadcast.event.as_str(),
                    intel_id.as_str()
                );
            }
        }
    }

    // Decide the next step:
    // - queue non-empty → schedule the static-gap timer, play a
    //   one-shot static burst, and let the timer system start the
    //   next broadcast when it fires.
    // - queue empty → open the idle yarn so the player sees
    //   "Radio: ..." with a close option.
    if queue.is_empty() {
        start_dialogue.write(StartDialogue {
            node: STATIC_YARN_NODE.to_string(),
            by: DialogueOwner::Radio,
        });
    } else if let Ok(transform) = radio_q.single() {
        spawn_oneshot_at(
            &mut commands,
            &sfx.static_burst,
            STATIC_BURST_VOLUME,
            transform.translation(),
        );
        commands.insert_resource(StaticGapTimer(Timer::new(
            Duration::from_secs_f32(STATIC_GAP_SECS),
            TimerMode::Once,
        )));
    }
}

/// Per-frame: tick the static-gap timer and pop the next broadcast
/// when it fires. Removes itself from the world on completion so
/// the next gap can be scheduled.
pub(super) fn tick_static_gap(
    mut commands: Commands,
    time: Res<Time>,
    listening: Res<ListeningToRadio>,
    timer: Option<ResMut<StaticGapTimer>>,
    mut queue: ResMut<RadioQueue>,
    mut active: ResMut<ActiveBroadcast>,
    sfx: Res<RadioSfx>,
    radio_q: Query<Entity, With<RadioMarker>>,
    mut start_dialogue: MessageWriter<StartDialogue>,
) {
    if !listening.active {
        return;
    }
    let Some(mut timer) = timer else {
        return;
    };
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    commands.remove_resource::<StaticGapTimer>();

    let Some(next) = queue.pop_next() else {
        // Queue became empty during the gap (shouldn't happen
        // normally but day-roll pruning could). Fall back to the
        // idle yarn so the player sees something.
        start_dialogue.write(StartDialogue {
            node: STATIC_YARN_NODE.to_string(),
            by: DialogueOwner::Radio,
        });
        return;
    };
    let node = next.yarn_node.clone();
    info!("radio: playing broadcast `{}`", next.event.as_str());
    active.0 = Some(next);
    if let Ok(radio_entity) = radio_q.single() {
        spawn_chatter(&mut commands, radio_entity, &sfx);
    }
    start_dialogue.write(StartDialogue {
        node,
        by: DialogueOwner::Radio,
    });
}

/// ESC handler: while listening, ESC signals an exit. Only acts
/// when the player isn't mid-broadcast — mid-broadcast, the
/// dialogue runner owns ESC (it's absorbed by the dialog). At
/// the idle / static-gap beats, the dialog panel is either
/// showing the idle node or absent, so ESC is free to trigger
/// an exit.
pub(super) fn handle_esc_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut listening: ResMut<ListeningToRadio>,
    active: Res<ActiveBroadcast>,
    mut exit_tx: MessageWriter<ExitListening>,
) {
    if !listening.active {
        return;
    }
    if active.0.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        listening.active = false;
        exit_tx.write(ExitListening);
    }
}

/// Clear every piece of radio-listening state on a fresh run.
/// Called from the app-level lifecycle reset so exiting to the
/// main menu mid-listen doesn't leak the `active` flag or the
/// queue into the next session.
pub fn reset_listening_state(
    mut listening: ResMut<ListeningToRadio>,
    mut active: ResMut<ActiveBroadcast>,
    mut queue: ResMut<RadioQueue>,
) {
    listening.active = false;
    active.0 = None;
    queue.entries.clear();
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
