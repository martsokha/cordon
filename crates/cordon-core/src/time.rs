use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Day(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Morning,
    Midday,
    Evening,
    Night,
}

impl Phase {
    pub fn next(self) -> Option<Phase> {
        match self {
            Phase::Morning => Some(Phase::Midday),
            Phase::Midday => Some(Phase::Evening),
            Phase::Evening => Some(Phase::Night),
            Phase::Night => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    pub day: Day,
    pub phase: Phase,
}

impl GameTime {
    pub fn new() -> Self {
        Self {
            day: Day(1),
            phase: Phase::Morning,
        }
    }

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
