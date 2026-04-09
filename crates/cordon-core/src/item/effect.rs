//! Effects applied by consumables, throwables, and relics.
//!
//! Three distinct shapes, one vocabulary:
//!
//! - [`TimedEffect`]      — a fire-and-forget change to a live
//!   resource, applied once (instant) or over a duration (per-minute).
//!   Produced by consumables and throwables.
//! - [`PassiveModifier`]  — an always-on flat stat modifier, applied
//!   while the source (relic) is equipped. Produced by relics.
//! - [`TriggeredEffect`]  — a reactive timed effect fired when an
//!   [`EffectTrigger`] condition is met. Produced by relics.
//!
//! The targets split into two disjoint enums too: [`ResourceTarget`]
//! for live resources that timed effects modify, [`StatTarget`] for
//! persistent stats that passive modifiers touch. The split makes it
//! impossible to write a passive HP-regen effect or a timed
//! resistance modifier — those wouldn't make sense.

use serde::{Deserialize, Serialize};

use crate::primitive::{Distance, Duration};

/// Live resources that timed effects modify.
///
/// Every variant corresponds to a per-entity numeric pool that
/// the effect dispatcher can mutate. Area effects (smoke clouds,
/// acid puddles) and status flags (bleeding, poison) don't live
/// here — they'd need different data shapes and their own
/// mechanisms, and will land alongside whichever feature
/// introduces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceTarget {
    /// Current HP. Positive values heal; negative values drain.
    Health,
    /// Current stamina.
    Stamina,
    /// Current hunger (higher = more sated).
    Hunger,
    /// Current radiation level carried by the character.
    /// Negative values reduce rads (anti-rad pills), positive
    /// increase (contaminated food, radioactive artifacts).
    RadiationLevel,
    /// Instantaneous damage dealt to the target (grenades,
    /// direct hits). Positive values deal damage. Distinct
    /// from [`Health`](Self::Health) with a negative value so
    /// "damage" and "healing" read distinctly at the call
    /// site.
    Damage,
}

/// Persistent stats that passive modifiers touch while their source
/// is equipped or active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatTarget {
    /// Flat addition to the carrier's max HP cap.
    MaxHealth,
    /// Flat addition to the carrier's max stamina cap.
    MaxStamina,
    /// Flat addition to the carrier's max hunger cap.
    MaxHunger,
    /// Flat addition to each resistance track.
    BallisticResistance,
    RadiationResistance,
    ChemicalResistance,
    ThermalResistance,
    ElectricResistance,
    GravitationalResistance,
}

/// A timed change to a live resource.
///
/// Consumables and throwables produce these on use. Triggered relic
/// effects also produce these when their trigger fires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimedEffect {
    /// The live resource this effect touches.
    pub target: ResourceTarget,
    /// Amount applied. Instant duration: total value. Non-instant:
    /// per-minute rate.
    pub value: f32,
    /// How long the effect runs once fired.
    /// [`Duration::INSTANT`] applies [`value`](Self::value) once;
    /// any non-instant duration applies it per minute for that long.
    pub duration: Duration,
    /// Area of effect radius. `None` means single-target (self or
    /// direct hit). Only meaningful for throwables.
    #[serde(default)]
    pub aoe: Option<Distance>,
}

/// A flat, always-on stat modifier produced by an equipped source
/// (typically a relic).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PassiveModifier {
    /// The persistent stat this modifier affects.
    pub target: StatTarget,
    /// Flat amount added to the stat while the source is equipped.
    /// Can be negative (a drawback on an otherwise-useful relic).
    pub value: f32,
}

/// When a [`TriggeredEffect`] fires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectTrigger {
    /// Fires when the carrier takes damage.
    OnHit,
    /// Fires edge-triggered when the carrier's HP drops below
    /// `max * threshold` (0.0–1.0).
    OnHpLow(f32),
    /// Fires on a recurring tick at the given interval while the
    /// source is equipped. Minute-grained like everything else;
    /// [`Duration::INSTANT`] is rejected by the sim at load time.
    Periodic(Duration),
}

/// A reactive effect: when [`trigger`](Self::trigger) fires, the
/// wrapped [`TimedEffect`] is applied to the carrier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TriggeredEffect {
    /// When this effect fires.
    pub trigger: EffectTrigger,
    /// The effect applied on each firing.
    pub effect: TimedEffect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_effect_deserializes_without_aoe() {
        let json = r#"{
            "target": "Health",
            "value": 50.0,
            "duration": "instant"
        }"#;
        let e: TimedEffect = serde_json::from_str(json).unwrap();
        assert_eq!(e.target, ResourceTarget::Health);
        assert_eq!(e.value, 50.0);
        assert_eq!(e.duration, Duration::INSTANT);
        assert_eq!(e.aoe, None);
    }

    #[test]
    fn timed_effect_deserializes_with_minute_duration() {
        let json = r#"{
            "target": "Bleeding",
            "value": 1.0,
            "duration": 5
        }"#;
        let e: TimedEffect = serde_json::from_str(json).unwrap();
        assert_eq!(e.duration.minutes(), 5);
    }

    #[test]
    fn passive_modifier_roundtrip() {
        let m = PassiveModifier {
            target: StatTarget::BallisticResistance,
            value: 20.0,
        };
        let json = serde_json::to_string(&m).unwrap();
        let parsed: PassiveModifier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn triggered_effect_roundtrip() {
        let t = TriggeredEffect {
            trigger: EffectTrigger::OnHit,
            effect: TimedEffect {
                target: ResourceTarget::Health,
                value: 5.0,
                duration: Duration::from_minutes(3),
                aoe: None,
            },
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: TriggeredEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn effect_trigger_on_hp_low_roundtrip() {
        let t = EffectTrigger::OnHpLow(0.3);
        let json = serde_json::to_string(&t).unwrap();
        let parsed: EffectTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn effect_trigger_periodic_roundtrip() {
        let t = EffectTrigger::Periodic(Duration::from_minutes(10));
        let json = serde_json::to_string(&t).unwrap();
        let parsed: EffectTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, t);
    }
}
