use serde::{Deserialize, Serialize};

use crate::entity::faction::FactionId;
use crate::economy::item::ItemKind;
use crate::entity::npc::NpcId;
use crate::world::time::Day;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    // Environmental
    Surge,
    Blowout,
    CreatureSwarm,
    HazardShift,
    PsiWave,

    // Economic
    SupplyDrop,
    Shortage(ItemKind),
    BlackMarketBust,
    NewRoute,
    TraderRivalry,

    // Faction
    FactionWar(FactionId, FactionId),
    FactionTruce(FactionId, FactionId),
    Coup(FactionId),
    FactionMission(FactionId),
    FactionPatrol(FactionId),
    MercenaryContract,
    DevotedPilgrimage,

    // Bunker
    Raid(FactionId),
    Inspection(FactionId),
    PowerOutage,
    Visitor,
    Infestation,
    Sabotage,
    BreakIn,

    // Personal
    RunnerLost(NpcId),
    Betrayal(NpcId),
    DebtCollector,
    WoundedStranger,
    OldFriend,
}

impl EventKind {
    pub fn category(&self) -> EventCategory {
        match self {
            EventKind::Surge
            | EventKind::Blowout
            | EventKind::CreatureSwarm
            | EventKind::HazardShift
            | EventKind::PsiWave => EventCategory::Environmental,

            EventKind::SupplyDrop
            | EventKind::Shortage(_)
            | EventKind::BlackMarketBust
            | EventKind::NewRoute
            | EventKind::TraderRivalry => EventCategory::Economic,

            EventKind::FactionWar(_, _)
            | EventKind::FactionTruce(_, _)
            | EventKind::Coup(_)
            | EventKind::FactionMission(_)
            | EventKind::FactionPatrol(_)
            | EventKind::MercenaryContract
            | EventKind::DevotedPilgrimage => EventCategory::Faction,

            EventKind::Raid(_)
            | EventKind::Inspection(_)
            | EventKind::PowerOutage
            | EventKind::Visitor
            | EventKind::Infestation
            | EventKind::Sabotage
            | EventKind::BreakIn => EventCategory::Bunker,

            EventKind::RunnerLost(_)
            | EventKind::Betrayal(_)
            | EventKind::DebtCollector
            | EventKind::WoundedStranger
            | EventKind::OldFriend => EventCategory::Personal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    Environmental,
    Economic,
    Faction,
    Bunker,
    Personal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    pub duration_days: u32,
    pub day_started: Day,
}

impl Event {
    pub fn is_expired(&self, current_day: Day) -> bool {
        current_day.0 >= self.day_started.0 + self.duration_days
    }

    pub fn days_remaining(&self, current_day: Day) -> u32 {
        let end = self.day_started.0 + self.duration_days;
        end.saturating_sub(current_day.0)
    }
}
