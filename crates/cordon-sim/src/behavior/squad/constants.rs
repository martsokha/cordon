//! Squad-AI tuning constants.

/// Baseline squad walk speed, in world units per second.
pub const SQUAD_WALK_SPEED: f32 = 30.0;

/// Walk speed when engaging — slightly faster so squads close the
/// gap on a hostile.
pub const ENGAGE_WALK_SPEED: f32 = 38.0;

/// Hold duration after arriving at a patrol waypoint.
pub const PATROL_HOLD_SECS: f32 = 6.0;

/// Distance threshold for "arrived at target" checks.
pub const ARRIVED_DIST: f32 = 12.0;

/// Follow distance for `Goal::Protect`: if the protecting squad's
/// leader is further than this from the protected squad's leader,
/// the protectors close the gap.
pub const PROTECT_FOLLOW_DIST: f32 = 40.0;

/// Formation system throttle.
pub const FORMATION_INTERVAL_SECS: f32 = 0.1;

/// Vision-scan throttle. The engagement system runs at most this
/// often regardless of frame rate.
pub const SCAN_INTERVAL_SECS: f32 = 0.1;

/// Coarse grid cell size for engagement's spatial index, in world
/// units. Bigger = fewer cells, more false positives per vision
/// check; smaller = more cells, more overhead.
pub const ENGAGEMENT_CELL_SIZE: f32 = 200.0;
