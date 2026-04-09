//! NPC type markers + the hidden-personality enum.
//!
//! There's no `Npc` data struct anymore — NPCs are Bevy entities
//! assembled from cordon-core component types (`NpcName`,
//! `Loadout`, `Experience`, `Credits`, `Trust`, `Loyalty`,
//! `Personality`) plus cordon-sim glue (`NpcMarker`,
//! `FactionId`, `NpcAttributes`, `Perks`, `Employment`,
//! `NpcBundle`). The generator in cordon-sim produces bundles
//! directly.
//!
//! What remains here is:
//!
//! - `Npc` — a phantom marker type used as the type parameter
//!   on [`Uid<Npc>`], so stable save-game IDs stay typed.
//! - `Role` / `Personality` — enum flavour types stored as
//!   fields inside components, not as components themselves.
//! - `NpcTemplate` — marker for NPC template IDs used in quest
//!   consequences.

use serde::{Deserialize, Serialize};

use crate::primitive::IdMarker;

/// Phantom marker for NPC-stable save-game IDs. Used as the
/// type parameter on `Uid<Npc>`. Has no fields — all the actual
/// NPC data lives on the Bevy entity as components.
pub struct Npc;

/// Marker for NPC template IDs (used in quest consequences).
pub struct NpcTemplate;
impl IdMarker for NpcTemplate {}

/// What role an employed NPC fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub enum Role {
    /// Goes into the Zone to scavenge, deliver, or gather intel.
    Runner,
    /// Stays at the bunker to deter theft, enable intimidation,
    /// and fight raids.
    Guard,
}

/// Core personality trait affecting negotiation behavior
/// (hidden from the player). Stored on entities as a field of
/// `NpcAttributes`, not as its own component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[derive(Serialize, Deserialize)]
pub enum Personality {
    /// Careful, slow to trust, thorough negotiator.
    #[default]
    Cautious,
    /// Confrontational, may escalate if refused.
    Aggressive,
    /// Straightforward, unlikely to scam.
    Honest,
    /// May lie about item quality or their situation.
    Deceptive,
    /// Willing to go back and forth on price.
    Patient,
    /// Makes snap decisions, may accept bad deals.
    Impulsive,
}
