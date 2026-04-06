//! Day-based time system for the game loop.
//!
//! Each in-game day has two periods: working hours (when trading,
//! visitors, and missions happen) and off hours (end-of-day
//! bookkeeping, events expire, new day begins).

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

/// Whether the bunker is open for business.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub enum Period {
    /// Trading, visitors, missions dispatched.
    #[default]
    Working,
    /// End-of-day: events expire, payroll, preparation for next day.
    Off,
}

/// Tracks the current day and period within the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// The current day number.
    pub day: Day,
    /// Current period of the day.
    pub period: Period,
}

impl GameTime {
    /// Create a new game time starting at Day 1, working hours.
    pub fn new() -> Self {
        Self {
            day: Day::FIRST,
            period: Period::Working,
        }
    }

    /// Advance to the next period. Working → Off → next day Working.
    pub fn advance(&mut self) {
        match self.period {
            Period::Working => self.period = Period::Off,
            Period::Off => {
                self.day = self.day.next();
                self.period = Period::Working;
            }
        }
    }
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new()
    }
}
