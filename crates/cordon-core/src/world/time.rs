//! Day/phase time system for the game loop.
//!
//! Each in-game day has four phases: Morning, Midday, Evening, Night.
//! The simulation advances one phase at a time.

use serde::{Deserialize, Serialize};

/// An in-game day, starting from day 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Serialize, Deserialize)]
pub struct Day(pub u32);

/// One of four phases within a day.
///
/// The day cycle follows: Morning → Midday → Evening → Night → next day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Preparation: check prices, dispatch runners, review intel.
    Morning,
    /// Trading: customers arrive, buy/sell/negotiate.
    Midday,
    /// Consequences: runners return, faction visits, events resolve.
    Evening,
    /// Management: upgrades, planning, end-of-day bookkeeping.
    Night,
}

impl Phase {
    /// Returns the next phase, or `None` if this is the last phase of the day.
    pub fn next(self) -> Option<Phase> {
        match self {
            Phase::Morning => Some(Phase::Midday),
            Phase::Midday => Some(Phase::Evening),
            Phase::Evening => Some(Phase::Night),
            Phase::Night => None,
        }
    }
}

/// Tracks the current day and phase within the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// The current day number.
    pub day: Day,
    /// The current phase within the day.
    pub phase: Phase,
}

impl GameTime {
    /// Create a new game time starting at Day 1, Morning.
    pub fn new() -> Self {
        Self {
            day: Day(1),
            phase: Phase::Morning,
        }
    }

    /// Advance to the next phase. If the current phase is Night,
    /// rolls over to the next day's Morning.
    pub fn advance(&mut self) {
        match self.phase.next() {
            Some(next) => self.phase = next,
            None => {
                self.day.0 += 1;
                self.phase = Phase::Morning;
            }
        }
    }
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new()
    }
}
