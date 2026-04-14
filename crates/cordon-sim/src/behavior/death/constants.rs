//! Tuning constants for corpse lifecycle.

/// How long corpses stay in the world (in-game minutes) before
/// despawning. The soft cleanup — looted corpses despawn earlier,
/// and the hard cap below is also enforced for sessions that don't
/// produce enough kills to age corpses out.
pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

/// Hard ceiling on dead NPC corpses kept in the world. When
/// exceeded, the oldest corpses (by `died_at`) are despawned even
/// if they haven't aged past [`CORPSE_PERSISTENCE_MINUTES`]. Keeps
/// entity counts bounded over long sessions where the time-based
/// cleanup hasn't fired yet.
pub const MAX_DEAD_NPCS: usize = 200;

/// Throttle for corpse-cleanup systems (corpse sweep, cap enforcer).
/// One pass per second is enough for gameplay and keeps the
/// lifecycle scan off the per-frame hot path.
pub const CLEANUP_INTERVAL_SECS: f32 = 1.0;
