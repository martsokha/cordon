//! Faction definitions.
//!
//! [`FactionDef`] is loaded from config. Faction relationships use
//! [`Relation`] from the primitives module.

use serde::{Deserialize, Serialize};

use super::name::NamePoolMarker;
use crate::item::ItemCategory;
use crate::primitive::{Id, IdMarker, Relation};

/// Marker for faction IDs.
pub struct Faction;
impl IdMarker for Faction {}

/// Which rank naming convention a faction uses.
///
/// All factions use the same 5-tier rank system, but the titles
/// differ. The naming scheme ID is the localization key prefix
/// for rank titles (e.g., `Military` → `"rank.military.1"` through
/// `"rank.military.5"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RankScheme {
    /// Grunt, Soldier, Veteran, Officer, Commander.
    /// Used by the Garrison.
    Military,
    /// Rookie, Seasoned, Hardened, Boss, Legend.
    /// Used by Drifters and the Syndicate.
    Loose,
    /// Pilgrim, Acolyte, Keeper, Prophet, Ascended.
    /// Used by the Devoted.
    Religious,
    /// Recruit, Researcher, Senior, Director, Council.
    /// Used by the Institute.
    Academic,
}

/// Faction definition loaded from config.
///
/// The [`id`](FactionDef::id) doubles as the localization key —
/// display name, philosophy, and structure descriptions are resolved
/// from localization files, not stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionDef {
    /// Unique identifier and localization key (e.g., `"garrison"`,
    /// `"drifters"`).
    pub id: Id<Faction>,
    /// Whether NPCs from this faction can be recruited as runners/guards.
    pub recruitable: bool,
    /// Which rank naming convention this faction uses.
    pub rank_scheme: RankScheme,
    /// Item categories this faction typically buys.
    pub buys: Vec<ItemCategory>,
    /// Item categories this faction typically sells.
    pub sells: Vec<ItemCategory>,
    /// Name pool ID used to generate NPC names for this faction.
    pub namepool: Id<NamePoolMarker>,
    /// Base relations with other factions.
    pub relations: Vec<(Id<Faction>, Relation)>,
}
