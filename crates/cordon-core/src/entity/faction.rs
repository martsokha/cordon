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
    /// Unique identifier and localization key (e.g.,
    /// `"faction_garrison"`, `"faction_drifters"`).
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
    /// Display color for this faction as a hex string (e.g.
    /// `"#6B8C4D"`). Used to tint settlement disks, NPC dots, and
    /// corpse markers on the map so a faction's footprint is
    /// recognizable at a glance. Parsed once at game-data load.
    pub color: String,
    /// Relative weight used when the daily spawner picks which
    /// faction a fresh squad belongs to. Higher weights spawn more
    /// often. Treat the values as relative — if garrison=25 and
    /// institute=5, garrison spawns five times as often.
    pub spawn_weight: u32,
    /// Base relations with other factions.
    pub relations: Vec<(Id<Faction>, Relation)>,
}
