//! Tuning constants for NPC movement.

/// Half-extent of the playable map AABB. NPC positions are clamped
/// to `±MAP_BOUND` so they can't walk off the world during combat or
/// formation moves.
pub const MAP_BOUND: f32 = 1500.0;
