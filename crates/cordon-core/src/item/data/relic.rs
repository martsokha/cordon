//! Relic item data (Zone anomalous objects).

use serde::{Deserialize, Serialize};

use crate::item::effect::{PassiveModifier, TriggeredEffect};
use crate::primitive::HazardType;

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
/// - `passive` resistance modifiers (`*Resistance` targets) are
///   folded into combat's damage resolution by
///   [`crate::item::Loadout::equipped_resistances`].
/// - `passive` max-pool modifiers (`MaxHealth`, `MaxStamina`,
///   `MaxHunger`) are applied by `cordon_sim::behavior::sync_pool_maxes`
///   whenever a carrier's loadout changes.
/// - `triggered` is **data-only** until the trigger dispatcher lands
///   (see the commit 5 plan in the world-sim branch). Relics can
///   declare reactive effects today, but nothing fires them yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelicData {
    /// Which hazard type births this relic. The spawner uses this
    /// to pick candidate relics for each anomaly area.
    pub origin: HazardType,

    /// Always-on stat modifiers applied while this relic is carried
    /// in a relic slot.
    #[serde(default)]
    pub passive: Vec<PassiveModifier>,

    /// Reactive effects fired on specific events while this relic is
    /// carried. Not yet wired to a runtime dispatcher — see the
    /// type-level doc.
    #[serde(default)]
    pub triggered: Vec<TriggeredEffect>,
}
