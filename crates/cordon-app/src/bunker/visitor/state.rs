//! Public visitor types: the queue, state machine, and message.

use std::collections::VecDeque;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::item::Item;
use cordon_core::primitive::Id;

/// A pending visitor: who they are and what yarn node to start when
/// the player admits them.
#[derive(Debug, Clone)]
pub struct Visitor {
    pub display_name: String,
    pub faction: Id<Faction>,
    /// Yarn node to start when the player admits this visitor.
    /// Resume-after-step-away is a conversation-level concern,
    /// not an NPC-level one, so it lives on
    /// [`VisitorState::Waiting`] rather than here — see the
    /// `<<step_away>>` command.
    pub yarn_node: String,
    /// Catalog identity of the visitor, when the visitor was
    /// spawned from a template NPC (the normal quest path). The
    /// bunker lifecycle uses this to dismiss the sim-side NPC
    /// at the same moment the bunker sprite despawns — so the
    /// sprite and the sim entity appear together and leave
    /// together. `None` for narrator-synthesized or legacy
    /// visitors that have no template-backed sim entity.
    pub template: Option<Id<NpcTemplate>>,
    /// Items this visitor is delivering as part of trade orders.
    /// Populated when the arrival carried a
    /// [`PendingDeliveryItems`](cordon_sim::plugin::prelude::PendingDeliveryItems)
    /// component; `<<deliver_order>>` pops one per call. Empty
    /// for non-delivery visitors.
    pub delivery_items: Vec<Id<Item>>,
}

/// FIFO queue of visitors waiting outside.
#[derive(Resource, Default, Debug)]
pub struct VisitorQueue(pub VecDeque<Visitor>);

/// Current door state. Drives the button visual, sprite spawning,
/// and camera lock.
///
/// `Inside` and `Waiting` both carry a live sprite entity; the
/// difference is whether the dialogue runner is active and the
/// player is locked out of movement.
#[derive(Resource, Debug, Clone)]
pub enum VisitorState {
    /// No one at the door. The button is dim.
    Quiet,
    /// A visitor is waiting outside. The button glows red.
    Knocking { visitor: Visitor },
    /// Player admitted the visitor. Sprite is spawned, dialogue
    /// runner is on a yarn node, and player movement + interaction
    /// are locked out until the dialogue ends.
    Inside { visitor: Visitor, sprite: Entity },
    /// The player stepped away from an active dialogue (via
    /// `<<step_away "node">>`). Sprite is still present and
    /// interactable, but player has full FPS control.
    /// Interacting with the sprite resumes dialogue at
    /// `resume_node`, which the conversation specified at
    /// step-away time. The visitor stays in this state until
    /// either side of the dialogue signals termination.
    Waiting {
        visitor: Visitor,
        sprite: Entity,
        resume_node: String,
    },
}

/// Sent by the bunker `interact` system when the player presses E
/// while a visitor is knocking. Handled by the lifecycle module.
#[derive(Message, Debug, Default, Clone, Copy)]
pub struct AdmitVisitor;

/// Set by the `<<step_away "node">>` yarn command, consumed by
/// the `dismiss_on_dialogue_complete` system on the next frame.
/// When present at the moment the dialogue ends, the visitor
/// transitions to [`VisitorState::Waiting`] with the carried
/// `resume_node` instead of being dismissed.
///
/// Stored as a resource rather than an event because the "did
/// yarn call step_away this conversation?" question is about
/// the state *at dialogue-end time*, not about an event stream
/// — and a resource's presence/absence is the idiomatic Bevy
/// way to answer that question.
#[derive(Resource, Debug)]
pub struct PendingStepAway {
    pub resume_node: String,
}
