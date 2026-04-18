//! Pill dose tracking.
//!
//! [`PlayerPills`] records the in-game moment the player last
//! took a dose from a bunker medication cluster. Written by the
//! cordon-app interaction system, read by quest conditions
//! (`DaysWithoutPills`) and the once-per-day interactable gate.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;

/// Tracks when the player last took pills. `None` means never,
/// which the quest system interprets as "no doses since game
/// start" so the "n days without pills" trigger can fire on a
/// fresh run.
///
/// Stored as a full [`GameTime`] rather than just
/// [`Day`](cordon_core::primitive::Day) so the "days without
/// pills" check measures real elapsed time (game minutes /
/// 1440), not a day-number diff. That way taking pills at 23:59
/// on day 1 doesn't count as a full day without pills at 00:01
/// on day 2.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PlayerPills {
    pub last_taken: Option<GameTime>,
}

impl PlayerPills {
    /// Whole 24-hour spans elapsed since the last dose, or since
    /// the [`GameTime::default`] origin when the player has never
    /// taken pills.
    pub fn days_without(&self, now: GameTime) -> u32 {
        let baseline = self.last_taken.unwrap_or_default();
        now.minutes_since(baseline) / (24 * 60)
    }

    /// Stamp a dose at the given moment.
    pub fn record_dose(&mut self, now: GameTime) {
        self.last_taken = Some(now);
    }
}
