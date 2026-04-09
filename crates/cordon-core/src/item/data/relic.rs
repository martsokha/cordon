//! Relic item data (Zone anomalous objects).

use serde::{Deserialize, Serialize};

use crate::item::effect::{PassiveModifier, TriggeredEffect};

/// Data for Zone relics with anomalous properties.
///
/// A relic has two effect lists: passive modifiers that contribute
/// flat stat deltas while equipped, and triggered effects that react
/// to events. Either can be empty — a pure passive relic (+20
/// ballistic) has an empty `triggered`, a pure reactive relic
/// (+10 HP/sec for 3s on hit) has an empty `passive`, a hybrid
/// relic can use both.
///
/// ## Runtime status
///
/// - `passive` resistance modifiers are folded into combat's
///   damage resolution by [`crate::item::Loadout::equipped_resistances`].
/// - `passive` max-pool modifiers (`MaxHealth`, `MaxStamina`) are
///   applied by `cordon_sim::behavior::sync_pool_maxes` whenever a
///   carrier's loadout changes.
/// - `triggered` is wired to `cordon_sim::effects` for `OnHit`,
///   `OnLowHealth`, and `Periodic`; other threshold variants are
///   defined but not yet fired (pending the unified pool-change
///   bus).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelicData {
    /// Always-on stat modifiers applied while this relic is carried
    /// in a relic slot.
    #[serde(default)]
    pub passive: Vec<PassiveModifier>,

    /// Reactive effects fired while this relic is carried.
    #[serde(default)]
    pub triggered: Vec<TriggeredEffect>,
}
