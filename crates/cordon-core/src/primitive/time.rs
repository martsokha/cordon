//! Day-based time system for the game loop.
//!
//! Time is tracked as day number + hour:minute.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// An in-game day, starting from day 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Day(NonZeroU32);

impl Day {
    /// Day 1.
    pub const FIRST: Self = Self(NonZeroU32::new(1).unwrap());

    /// Create a day from a raw value. Panics if zero.
    pub fn new(value: u32) -> Self {
        Self(NonZeroU32::new(value).expect("day must be >= 1"))
    }

    /// Get the raw day number.
    pub fn value(self) -> u32 {
        self.0.get()
    }

    /// Advance to the next day.
    pub fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("day overflow"))
    }
}

/// Tracks the current day and time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// The current day number.
    pub day: Day,
    /// Hour of day (0–23).
    pub hour: u8,
    /// Minute of hour (0–59).
    pub minute: u8,
}

impl GameTime {
    /// Create a new game time starting at Day 1, 08:00.
    pub fn new() -> Self {
        Self {
            day: Day::FIRST,
            hour: 8,
            minute: 0,
        }
    }

    /// Advance time by the given number of minutes.
    pub fn advance_minutes(&mut self, minutes: u32) {
        let total = self.hour as u32 * 60 + self.minute as u32 + minutes;
        self.hour = ((total / 60) % 24) as u8;
        self.minute = (total % 60) as u8;
        let days = total / (24 * 60);
        for _ in 0..days {
            self.day = self.day.next();
        }
    }

    /// Advance time by the given number of hours.
    pub fn advance_hours(&mut self, hours: u32) {
        self.advance_minutes(hours * 60);
    }

    /// Formatted time string (e.g., "08:00").
    pub fn time_str(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// Normalized time of day (0.0 = midnight, 0.5 = noon, 1.0 = midnight).
    pub fn day_progress(&self) -> f32 {
        (self.hour as f32 + self.minute as f32 / 60.0) / 24.0
    }

    /// Whether it's daytime (6:00–21:00).
    pub fn is_day(&self) -> bool {
        self.hour >= 6 && self.hour < 21
    }

    /// Total minutes elapsed since the first moment of [`Day::FIRST`]
    /// at 00:00. Used for cheap duration arithmetic — subtract two
    /// snapshots to get the difference in minutes.
    pub fn total_minutes(&self) -> u64 {
        let days = (self.day.value() - 1) as u64;
        days * 24 * 60 + self.hour as u64 * 60 + self.minute as u64
    }

    /// Game-minutes elapsed from `earlier` to `self`. Returns 0 if
    /// `earlier` is in the future of `self` (clock running
    /// backwards is not expected but handled defensively).
    pub fn minutes_since(&self, earlier: GameTime) -> u32 {
        let now = self.total_minutes();
        let then = earlier.total_minutes();
        now.saturating_sub(then).min(u32::MAX as u64) as u32
    }
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new()
    }
}
