//! Relic item data (Zone anomalous objects).

use serde::{Deserialize, Serialize};

use crate::item::effect::Effect;

/// Stability state of a relic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicStability {
    /// Contained properly, safe to store and sell.
    Stable,
    /// Not contained, degrades over time, may harm handler.
    Unstable,
    /// Depleted or damaged, minimal value.
    Inert,
}

/// Data for Zone relics with anomalous properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelicData {
    /// Default stability when found. Affects storage requirements.
    pub default_stability: RelicStability,
    /// Passive effects while carried. Applied continuously to the
    /// carrier. If an effect has an [`aoe`](Effect::aoe), it also
    /// affects nearby characters. Duration is ignored.
    pub carried_effects: Vec<Effect>,
}
