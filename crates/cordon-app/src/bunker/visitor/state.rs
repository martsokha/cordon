//! Public visitor types: the queue, state machine, and message.

use std::collections::VecDeque;

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;

/// A pending visitor: who they are and what yarn node to start when
/// the player admits them.
#[derive(Debug, Clone)]
pub struct Visitor {
    pub display_name: String,
    pub faction: Id<Faction>,
    pub yarn_node: String,
}

/// FIFO queue of visitors waiting outside.
#[derive(Resource, Default, Debug)]
pub struct VisitorQueue(pub VecDeque<Visitor>);

/// Current door state. Drives the button visual, sprite spawning,
/// and camera lock.
#[derive(Resource, Debug, Clone)]
pub enum VisitorState {
    /// No one at the door. The button is dim.
    Quiet,
    /// A visitor is waiting outside. The button glows red.
    Knocking { visitor: Visitor },
    /// Player admitted the visitor. Sprite is spawned and dialogue
    /// runner is on a yarn node.
    Inside { visitor: Visitor, sprite: Entity },
}

/// Sent by the bunker `interact` system when the player presses E
/// while a visitor is knocking. Handled by the lifecycle module.
#[derive(Message, Debug, Default, Clone, Copy)]
pub struct AdmitVisitor;
