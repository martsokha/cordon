//! 2D location on the Zone map.

use serde::{Deserialize, Serialize};

use super::distance::Distance;

/// A 2D position on the Zone map.
///
/// Uses floating-point coordinates in arbitrary map units.
/// Used for area positions, runner tracking, bunker location, etc.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

impl Location {
    /// Origin point (0, 0).
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    /// Create a new location.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to another location.
    pub fn distance_to(self, other: Self) -> Distance {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        Distance::new((dx * dx + dy * dy).sqrt())
    }

    /// Whether a point is within a given distance of this location.
    pub fn within_radius(self, center: Self, radius: Distance) -> bool {
        self.distance_to(center).value() <= radius.value()
    }
}
