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
//! - [`bridge::enqueue_talk_dialogue`] — when a quest enters a
//!   `Talk` stage, either emits a `SpawnNpcRequest` (for template
//!   NPCs — the sim then walks the NPC to the bunker and a
//!   `BunkerArrival` pushes the `Visitor` onto the bunker queue),
//!   pushes a synthesized `Visitor` for string-tagged quests, or
//!   writes `StartDialogue` for narrator-only `Talk` stages.
//! - [`bridge::on_dialogue_completed`] — observer on
//!   `NodeCompleted` that matches the completed yarn node
//!   against the active quests' current Talk stages, drains
//!   `$quest_*` Yarn variables into the active quest's flags,
//!   emits `TalkCompleted` back to the sim, and clears the
//!   in-flight slot so the next `Talk` stage can dispatch.
//! - [`arrival::handle_bunker_arrival`] / [`arrival::handle_home_arrival`]
//!   — view-layer glue between the sim's travel messages and the
//!   bunker's visitor queue / idle-squad setup.

mod arrival;
mod bridge;

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::FactionSettlements;

pub use self::bridge::DialogueInFlight;

pub struct QuestBridgePlugin;

impl Plugin for QuestBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueInFlight>();
        // Runs in any PlayingState — template-NPC travel and
        // visitor-queue pushes must happen even while the player is
        // at the laptop, so the door alarm rings and quests can chain
        // without the player having to be at the desk. The dialogue
        // UI is still gated to PlayingState::Bunker, so the actual
        // conversation only renders when the player returns.
        //
        // The resource-exists gate keeps the system off during the
        // early load frames before GameDataResource is inserted.
        app.add_systems(
            Update,
            bridge::enqueue_talk_dialogue.run_if(resource_exists::<GameDataResource>),
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
