//! Per-NPC loyalty to their faction.

use serde::{Deserialize, Serialize};

/// How loyal an NPC is to their nominal faction. Range
/// `[-1.0, 1.0]`: -1.0 = ready to defect, 0.0 = indifferent,
/// 1.0 = zealous. Affects whether they'll take jobs against
/// their own faction, whether they can be bribed, and whether
/// faction reputation events drag them along with the group.
///
/// Stored on entities as a field of `NpcAttributes`.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Loyalty(pub f32);
