//! Zone events that affect the game world.
//!
//! Events are rolled daily by the simulation and tracked with a duration.
//! Each event has a kind (what happened), a duration, and a start day.

use serde::{Deserialize, Serialize};

use crate::economy::item::ItemKind;
use crate::object::id::{Id, Uid};
use crate::world::time::Day;

/// What kind of event is occurring.
///
/// Events are grouped into categories: environmental, economic, faction,
/// bunker, and personal. Some events are parameterized by faction ID
/// or NPC UID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    // -- Environmental --
    /// All outdoor activity halted. Runners at risk. Relics shift.
    Surge,
    /// Severe surge. Major casualties. Prices spike.
    Blowout,
    /// Dangerous creatures flood a sector. Weapon demand spikes.
    CreatureSwarm,
    /// Hazard fields move. Routes change. Maps outdated.
    HazardShift,
    /// Psychic disturbance. Erratic NPC behavior. Psi-gear demand surges.
    PsiWave,

    // -- Economic --
    /// Military convoy lost. Cheap goods flood the market.
    SupplyDrop,
    /// A category of goods becomes scarce. Prices spike.
    Shortage(ItemKind),
    /// Authorities crack down on illegal goods.
    BlackMarketBust,
    /// Safe path opens to a dangerous sector.
    NewRoute,
    /// Another trader undercuts your prices.
    TraderRivalry,

    // -- Faction (parameterized by faction ID) --
    /// Two factions clash. Soldiers buy urgently. Collateral damage.
    FactionWar(Id, Id),
    /// Two hostile factions temporarily cooperate.
    FactionTruce(Id, Id),
    /// Faction leadership changes. Standing partially resets.
    Coup(Id),
    /// A faction gives the player a timed task.
    FactionMission(Id),
    /// Faction soldiers "inspect" the bunker for contraband.
    FactionPatrol(Id),
    /// Mercenaries hired to hit a target nearby.
    MercenaryContract,
    /// Devoted zealots move through sectors en masse.
    DevotedPilgrimage,

    // -- Bunker --
    /// Armed attack on the bunker. Outcome depends on guards and defenses.
    Raid(Id),
    /// Official inspection. Contraband confiscated unless hidden.
    Inspection(Id),
    /// Electronics go down. Food spoils faster.
    PowerOutage,
    /// Someone asks for shelter. Help costs resources.
    Visitor,
    /// Vermin or creatures damage stored items.
    Infestation,
    /// Equipment tampered with. Radio jammed, stock poisoned.
    Sabotage,
    /// Overnight robbery attempt. Defenses determine outcome.
    BreakIn,

    // -- Personal (parameterized by NPC UID) --
    /// A runner goes missing in the field.
    RunnerLost(Uid),
    /// A trusted NPC steals from you or feeds false intel.
    Betrayal(Uid),
    /// Someone claims you owe them.
    DebtCollector,
    /// A scavenger collapses at your door.
    WoundedStranger,
    /// An NPC from the past returns with an opportunity.
    OldFriend,
}

impl EventKind {
    /// Which category this event belongs to.
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

/// Broad category for event grouping and scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventCategory {
    /// Weather, surges, hazard shifts, creature activity.
    Environmental,
    /// Supply/demand shifts, shortages, market disruptions.
    Economic,
    /// Wars, truces, patrols, coups.
    Faction,
    /// Raids, inspections, power outages, infestations.
    Bunker,
    /// Runner losses, betrayals, visitors.
    Personal,
}

/// An active event in the game world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// What kind of event this is.
    pub kind: EventKind,
    /// How many days this event lasts.
    pub duration_days: u32,
    /// Which day this event started.
    pub day_started: Day,
}

impl Event {
    /// Whether this event has expired (current day is past its end).
    pub fn is_expired(&self, current_day: Day) -> bool {
        current_day.0 >= self.day_started.0 + self.duration_days
    }

    /// How many days remain until this event expires.
    pub fn days_remaining(&self, current_day: Day) -> u32 {
        let end = self.day_started.0 + self.duration_days;
        end.saturating_sub(current_day.0)
    }
}
