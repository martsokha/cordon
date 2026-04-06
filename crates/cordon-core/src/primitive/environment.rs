//! Environment types for areas and locations.

use serde::{Deserialize, Serialize};

/// Whether a location is indoors, outdoors, or underground.
///
/// Affects surge behavior (outdoor runners at risk, indoor ones safe),
/// weather effects, radio contact, and visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Open air. Affected by surges, weather, and visibility.
    Outdoor,
    /// Inside a building. Sheltered from surges and weather.
    Indoor,
    /// Below ground. No radio contact, sheltered, claustrophobic.
    Underground,
}
