//! Combat messages (events) consumed by the visual + audio layers
//! and by the effect dispatcher.

use bevy::prelude::*;
use cordon_core::item::ResourceTarget;

/// A weapon discharged from `from` toward `to`. The visual layer
/// renders a tracer; the audio layer plays a gunshot. Emitted
/// at most once per shooter per frame by `resolve_combat`.
#[derive(Message, Debug, Clone, Copy)]
pub struct ShotFired {
    pub shooter: Entity,
    pub from: Vec2,
    pub to: Vec2,
}

/// Emitted whenever an NPC's pool (`Health`, `Stamina`, or
/// `Corruption`) changes value.
///
/// Produced by combat's damage apply and by the effect dispatcher
/// whenever it mutates a pool via a timed effect or instant
/// consumable. Downstream systems use this to detect threshold
/// crossings (`prev > threshold && current <= threshold`) without
/// storing their own previous-state tracking.
///
/// `pool` is never [`ResourceTarget::Damage`] — damage is
/// normalised to a `Health` decrease before the event is written.
#[derive(Message, Debug, Clone, Copy)]
pub struct NpcPoolChanged {
    /// The entity whose pool changed.
    pub entity: Entity,
    /// Which pool changed.
    pub pool: ResourceTarget,
    /// Pool current value before the change.
    pub prev: u32,
    /// Pool current value after the change.
    pub current: u32,
    /// Pool max at the time the event was emitted. Used to
    /// compute threshold crossings without a second component
    /// lookup in the subscriber.
    pub max: u32,
}

impl NpcPoolChanged {
    /// Signed delta `(current - prev)`. Negative = drain.
    pub fn delta(&self) -> i32 {
        self.current as i32 - self.prev as i32
    }
}
