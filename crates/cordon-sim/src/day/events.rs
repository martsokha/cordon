//! Day-cycle messages.

use bevy::prelude::*;
use cordon_core::primitive::Day;

/// In-game day advanced. Fires exactly once per day rollover from
/// `detect_day_rollover`; per-day work (daily event rolls, faction
/// reactions, event expiry) runs as separate systems gated on this
/// message.
#[derive(Message, Debug, Clone, Copy)]
pub struct DayRolled {
    pub new_day: Day,
}
