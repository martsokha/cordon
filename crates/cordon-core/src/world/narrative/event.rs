//! Zone events that affect the game world.
//!
//! [`EventDef`] is loaded from JSON config files. [`ActiveEvent`] is a
//! runtime instance of an event currently affecting the world.
//!
//! Events are data-driven: their category, base probability, duration
//! range, and parameters are all defined in config. The sim rolls
//! daily for each event based on its probability and world state.

use serde::{Deserialize, Serialize};

use super::consequence::Consequence;
use super::intel::Intel;
use crate::entity::faction::Faction;
use crate::primitive::{Day, Id, IdMarker};
use crate::world::area::Area;

/// Marker for event definition IDs.
pub struct Event;
impl IdMarker for Event {}

/// Broad category for event grouping and scheduling.
///
/// Each category has its own base roll probability per day, modified
/// by world state (Zone instability, market stability, faction tensions,
/// security level, narrative flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// Weather, surges, hazard shifts, creature activity.
    Environmental,
    /// Supply/demand shifts, shortages, market disruptions.
    Economic,
    /// Wars, truces, patrols, coups, faction-specific visitors.
    Faction,
    /// Raids, inspections, power outages, infestations.
    Bunker,
    /// Runner losses, betrayals, special visitors, encounters.
    Personal,
}

/// Optional radio broadcast tied to an event. When present the radio
/// announces the event to the player after a configurable delay and
/// can grant intel entries on broadcast.
///
/// Display text is derived from the event's own ID: `event.{id}.radio`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioEntry {
    /// Game-minutes after event start before the broadcast.
    /// Zero means the broadcast fires immediately.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub delay_minutes: u32,
    /// When true (default), the broadcast fires once — if the
    /// radio is off at that moment, the player misses it. When
    /// false, the broadcast stays queued until the radio is on
    /// or the day ends.
    #[serde(default = "default_true")]
    pub missable: bool,
    /// Intel entries this broadcast unlocks for the player.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants_intel: Vec<Id<Intel>>,
}

fn default_true() -> bool {
    true
}

/// An event definition loaded from config.
///
/// Defines what an event is, how likely it is to occur, how long it
/// lasts, what direct effects it has, and what parameters it carries.
/// The [`id`](EventDef::id) doubles as the localization key.
///
/// # Examples from config
///
/// - `"surge"`: category Environmental, base_probability 0.08, duration 1..1,
///   consequences: [DangerModifier { area: None, delta: 0.3 }]
/// - `"faction_war"`: category Faction, base_probability 0.05, duration 3..7,
///   involves two faction IDs (resolved at runtime by the sim)
/// - `"garrison_commander_visit"`: category Faction, base_probability 0.1
/// - `"information_seller"`: category Personal, base_probability 0.03
/// - `"intelligent_creature"`: category Environmental, base_probability 0.01
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDef {
    /// Unique identifier and localization key (e.g., `"surge"`, `"faction_war"`).
    pub id: Id<Event>,
    /// Which category this event belongs to.
    pub category: EventCategory,
    /// Base probability of this event occurring per day (0.0–1.0).
    /// Modified at runtime by escalation and world state.
    pub base_probability: f32,
    /// Minimum duration in days.
    pub min_duration: u8,
    /// Maximum duration in days.
    pub max_duration: u8,
    /// Maximum simultaneous instances of this event. `None` means unlimited.
    pub max_instances: Option<u8>,
    /// Area IDs this event can target. Empty means zone-wide.
    pub target_areas: Vec<Id<Area>>,
    /// Faction IDs involved in this event. Empty means no faction tie.
    /// For events like wars or patrols, the sim picks from this list
    /// or from all factions if empty.
    pub involved_factions: Vec<Id<Faction>>,
    /// Minimum day before this event can first occur. Prevents
    /// endgame events from firing on day 1.
    pub earliest_day: Day,
    /// Direct consequences when this event fires (e.g., danger modifier,
    /// price changes, standing shifts). Applied immediately by the sim.
    pub consequences: Vec<Consequence>,
    /// IDs of events that chain from this one (e.g., surge → relic rush).
    pub chain_events: Vec<Id<Event>>,
    /// Optional radio broadcast. Events without this field happen
    /// silently in the background; events with it are announced to
    /// the player via the radio and can grant intel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio: Option<RadioEntry>,
}

/// An active event instance in the game world.
///
/// Created by the sim when an event fires. Tracks which event def
/// it came from, when it started, how long it lasts, and any
/// runtime parameters (which factions are involved, which area
/// is affected, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEvent {
    /// ID of the [`EventDef`] this is an instance of.
    pub def_id: Id<Event>,
    /// Which day this event started.
    pub day_started: Day,
    /// How many days this event lasts (rolled from def's min/max range).
    pub duration_days: u8,
    /// Faction IDs involved in this specific instance.
    pub involved_factions: Vec<Id<Faction>>,
    /// Area ID this event is targeting.
    /// Zone-wide events have `None`; area-specific events have `Some`.
    pub target_area: Option<Id<Area>>,
}

impl ActiveEvent {
    /// Whether this event has expired (current day is past its end).
    pub fn is_expired(&self, current_day: Day) -> bool {
        current_day.value() >= self.day_started.value() + self.duration_days as u32
    }

    /// How many days remain until this event expires.
    pub fn days_remaining(&self, current_day: Day) -> u32 {
        let end = self.day_started.value() + self.duration_days as u32;
        end.saturating_sub(current_day.value())
    }
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}
