//! Quest ↔ dialogue bridge.
//!
//! The quest runtime lives in `cordon-sim` and is pure state
//! machinery: trigger dispatch, objective evaluation, outcome
//! application. It doesn't know anything about visitors,
//! dialogue runners, or Yarn variable storage.
//!
//! This module is the thin layer that connects quest state to
//! the dialogue system:
//!
//! - [`enqueue_talk_visitors`] — when a quest enters a `Talk`
//!   stage, push a [`Visitor`] onto the bunker's visitor queue
//!   tagged with the quest ID so the dialogue driver can route
//!   the reply back.
//! - [`on_dialogue_completed`] — observer on `DialogueCompleted`
//!   that drains `$quest_*` Yarn variables into the active
//!   quest's flags and calls [`engine::advance_after_talk`] to
//!   move the quest forward.
//!
//! Quests without a giver NPC (narrator-only `Talk` stages) are
//! *not* handled here yet — they need a direct dialogue start
//! without the visitor queue. The first quest uses a giver
//! every time, so narrator-only stages can wait.

mod arrival;
mod bridge;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::FactionSettlements;

use self::bridge::DialogueInFlight;
use crate::PlayingState;

pub struct QuestBridgePlugin;

impl Plugin for QuestBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueInFlight>();
        app.add_systems(
            Update,
            bridge::enqueue_talk_dialogue.run_if(in_state(PlayingState::Bunker)),
        );
        app.add_observer(bridge::on_dialogue_completed);
        app.add_systems(
            Update,
            (arrival::handle_bunker_arrival, arrival::handle_home_arrival)
                .run_if(resource_exists::<GameDataResource>)
                .run_if(resource_exists::<FactionSettlements>),
        );
    }
}
