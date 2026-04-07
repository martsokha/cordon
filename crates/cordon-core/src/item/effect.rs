//! Effects applied by consumables, throwables, and relics.

use serde::{Deserialize, Serialize};

use crate::primitive::{Distance, Duration};

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
/// per second over a [`Duration`]. Instant effects use
/// [`Duration::INSTANT`].
///
/// # Examples
///
/// ```ignore
/// // Medkit: instant +50 HP
/// Effect { target: EffectTarget::Health, value: 50.0, duration: Duration::INSTANT, aoe: None }
///
/// // Anti-rad pills: -5 rad/sec for 10 seconds
/// Effect { target: EffectTarget::Radiation, value: -5.0, duration: Duration::new(10), aoe: None }
///
/// // Frag grenade: -30 dmg/sec for 2 sec in 5m radius
/// Effect { target: EffectTarget::Damage, value: -30.0, duration: Duration::new(2), aoe: Some(5.0) }
///
/// // Smoke grenade: smoke for 15 sec in 8m radius
/// Effect { target: EffectTarget::Smoke, value: 1.0, duration: Duration::new(15), aoe: Some(8.0) }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    /// What this effect modifies.
    pub target: EffectTarget,
    /// Amount applied per second (or total if instant). Positive = beneficial, negative = harmful.
    pub value: f32,
    /// How long this effect lasts. [`Duration::INSTANT`] means applied once.
    pub duration: Duration,
    /// Area of effect radius. `None` means single-target (self or direct hit).
    pub aoe: Option<Distance>,
}
