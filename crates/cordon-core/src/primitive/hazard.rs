//! Environmental hazard types in the Zone.

use serde::{Deserialize, Serialize};

/// What kind of environmental hazard is present.
///
/// Determines what protective gear runners need. Areas, events,
/// and missions can reference hazard types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HazardType {
    /// Toxic gas, industrial waste. Needs gas mask / hazard suit.
    Chemical,
    /// Heat anomalies, burning ground. Sudden damage spikes.
    Thermal,
    /// Lightning anomalies, tesla fields. Equipment damage, stun.
    Electric,
    /// Crushing or lifting anomalies. Instant kill zones.
    Gravitational,
}
