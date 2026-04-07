//! Gameplay tuning knobs.
//!
//! All numbers that control feel (distances, timings, thresholds,
//! probabilities) live here so we can tune the sim from one place
//! without hunting through systems.

/// Half-extent of the playable map AABB. NPC positions are clamped
/// to `±MAP_BOUND` so they can't walk off the world during combat or
/// formation moves.
pub const MAP_BOUND: f32 = 1500.0;

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

/// How long corpses stay in the world (in-game minutes) before
/// despawning. At the default 1 week, corpses naturally clean up
/// without the player needing to loot them.
pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

/// Squad-lifecycle cleanup throttle.
pub const CLEANUP_INTERVAL_SECS: f32 = 1.0;

/// Reach for corpse looting: NPCs within this distance can loot.
pub const LOOT_REACH: f32 = 12.0;

/// Time between individual item transfers while looting.
pub const LOOT_INTERVAL_SECS: f32 = 0.4;

/// Spawn attempts per anomaly area per day rollover. Each attempt
/// is an independent probability roll, so on average the system
/// spawns `ATTEMPTS_PER_AREA * SPAWN_PROBABILITY` relics per day
/// per area, capped by intensity-tier.
///
/// At the current 2 attempts × 0.6 probability = ~1.2 relics per
/// anomaly per day, which trends to cap over 2-5 in-game days
/// depending on intensity tier. Raise attempts for faster ramp-up,
/// raise probability for smoother day-to-day variance.
pub const RELIC_ATTEMPTS_PER_AREA: u32 = 2;

/// Probability per attempt that a relic is spawned this day. See
/// [`RELIC_ATTEMPTS_PER_AREA`] for the derivation.
pub const RELIC_SPAWN_PROBABILITY: f32 = 0.6;

/// Pickup reach for relics: a scavenging squad leader within this
/// distance of a world relic automatically collects it on the next
/// loot tick.
pub const RELIC_PICKUP_REACH: f32 = 16.0;

/// Earliest daytime fraction at which spawn waves can fire. 0.25 =
/// 06:00. Waves are spread between `SPAWN_DAY_START` and
/// `SPAWN_DAY_END` so population ramps up during waking hours.
pub const SPAWN_DAY_START: f32 = 0.25;

/// Latest daytime fraction at which spawn waves can fire. 0.875 =
/// 21:00.
pub const SPAWN_DAY_END: f32 = 0.875;
