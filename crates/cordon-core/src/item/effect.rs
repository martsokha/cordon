//! Effects applied by consumables, throwables, and relics.

use serde::{Deserialize, Serialize};

use crate::primitive::duration::Duration;

/// What stat or state an effect modifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectTarget {
    /// Restores health points.
    Health,
    /// Reduces radiation level.
    Radiation,
    /// Sates hunger (higher = more sated).
    Hunger,
    /// Sates thirst.
    Thirst,
    /// Deals damage to a target.
    Damage,
    /// Reduces stamina drain.
    Stamina,
    /// Stops bleeding.
    Bleeding,
    /// Removes poison/toxin effects.
    Poison,
    /// Obscures vision in an area (smoke grenades).
    Smoke,
}

/// An effect applied by a consumable, throwable, or relic.
///
/// Each effect modifies a single [`EffectTarget`] by a given value
/// per second over a [`Duration`]. Effects with `Duration::INSTANT`
/// apply their full value once immediately.
///
/// # Examples
///
/// - Medkit: `Effect { target: Health, value: 50.0, duration: Duration::INSTANT, aoe: None }`
/// - Anti-rad pills: `Effect { target: Radiation, value: -5.0, duration: Duration(10), aoe: None }`
/// - Frag grenade: `Effect { target: Damage, value: -30.0, duration: Duration(2), aoe: Some(5.0) }`
/// - Smoke grenade: `Effect { target: Smoke, value: 1.0, duration: Duration(15), aoe: Some(8.0) }`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    /// What this effect modifies.
    pub target: EffectTarget,
    /// Amount applied per second. Positive = beneficial, negative = harmful.
    pub value: f32,
    /// How long this effect lasts.
    pub duration: Duration,
    /// Area of effect radius in meters. `None` means single-target (self or direct hit).
    pub aoe: Option<f32>,
}
