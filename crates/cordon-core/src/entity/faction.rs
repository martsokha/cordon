use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactionId {
    Order,
    Collective,
    Syndicate,
    Garrison,
    Institute,
    Drifters,
    Mercenaries,
    Devoted,
}

impl FactionId {
    pub const ALL: [FactionId; 8] = [
        FactionId::Order,
        FactionId::Collective,
        FactionId::Syndicate,
        FactionId::Garrison,
        FactionId::Institute,
        FactionId::Drifters,
        FactionId::Mercenaries,
        FactionId::Devoted,
    ];

    pub fn is_recruitable(self) -> bool {
        matches!(
            self,
            FactionId::Drifters | FactionId::Syndicate | FactionId::Mercenaries
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing(i8);

impl Standing {
    pub const MIN: i8 = -100;
    pub const MAX: i8 = 100;

    pub fn new(value: i8) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn neutral() -> Self {
        Self(0)
    }

    pub fn value(self) -> i8 {
        self.0
    }

    pub fn apply(&mut self, delta: i8) {
        self.0 = (self.0 as i16 + delta as i16).clamp(Self::MIN as i16, Self::MAX as i16) as i8;
    }

    pub fn is_hostile(self) -> bool {
        self.0 <= -50
    }

    pub fn is_unfriendly(self) -> bool {
        self.0 > -50 && self.0 < 0
    }

    pub fn is_neutral(self) -> bool {
        self.0 >= 0 && self.0 < 50
    }

    pub fn is_friendly(self) -> bool {
        self.0 >= 50 && self.0 < 80
    }

    pub fn is_allied(self) -> bool {
        self.0 >= 80
    }
}

impl Default for Standing {
    fn default() -> Self {
        Self::neutral()
    }
}

/// Static relationship between two factions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRelation {
    pub a: FactionId,
    pub b: FactionId,
    pub base_standing: Standing,
}
