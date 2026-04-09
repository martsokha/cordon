//! Per-NPC trust toward the player.

use serde::{Deserialize, Serialize};

/// How much an NPC trusts the player. Range `[-1.0, 1.0]`:
/// -1.0 = hostile, 0.0 = neutral, 1.0 = fully trusted.
///
/// Hidden from the player directly — the UI only hints at trust
/// through behaviour changes. Climbs with successful trades, good
/// deals, and faction-reputation events; drops with betrayals,
/// bad deals, and hostile faction actions.
///
/// Stored on entities as a field of `NpcAttributes`, not as its
/// own component, so a single query touch gets the whole "how
/// does this NPC feel" bundle instead of three separate ones.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Trust(pub f32);
