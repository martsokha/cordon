//! Zone events that affect the game world.
//!
//! [`EventDef`] is loaded from JSON config files. [`ActiveEvent`] is a
//! runtime instance of an event currently affecting the world.
//!
//! Events are data-driven: their spawn weight, duration range, and
//! parameters are all defined in config. The sim rolls daily for each
//! event that carries a spawn weight; events without one are
//! quest-only — they never roll and can only be spawned via the
//! `TriggerEvent` consequence.

use serde::{Deserialize, Serialize};

use super::consequence::Consequence;
use super::intel::Intel;
use crate::entity::faction::Faction;
use crate::primitive::{Day, Id, IdMarker};
use crate::world::area::Area;

/// Marker for event definition IDs.
pub struct Event;
impl IdMarker for Event {}

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
    /// When true (default), the broadcast expires at end of day if
    /// the player hasn't listened to it yet. When false, the
    /// broadcast stays in the radio queue indefinitely until the
    /// player tunes in.
    ///
    /// Broadcasts deliver into a queue on the radio prop, not
    /// directly into the player's ears — intel grants fire only
    /// when the player turns the radio on and the dialogue plays
    /// through. `missable` only changes how long the queued item
    /// waits before being dropped.
    #[serde(default = "default_true")]
    pub missable: bool,
    /// Yarn node that runs when the player listens to this
    /// broadcast. The node's lines are what the player reads; intel
    /// grants on the yarn node completing (not on queue delivery).
    /// Required — every broadcast must have authored content.
    pub yarn_node: String,
    /// Intel entry this broadcast unlocks for the player. Granted
    /// when the yarn node completes, not when the broadcast queues.
    /// `None` for flavor-only broadcasts (no new intel attached).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grants_intel: Option<Id<Intel>>,
    /// When true, this broadcast is encrypted traffic: it only
    /// reaches the player if an installed upgrade grants
    /// [`UpgradeEffect::ListeningDevice`](crate::entity::bunker::UpgradeEffect::ListeningDevice).
    /// Without the device the broadcast is silently dropped — no
    /// intel grant, no audio, no toast. Missable broadcasts follow
    /// the usual missable rules on top of this (install the device
    /// before the event fires, or miss it).
    #[serde(default, skip_serializing_if = "is_false")]
    pub encrypted: bool,
}

fn default_true() -> bool {
    true
}

/// An event definition loaded from config.
///
/// Defines what an event is, how likely it is to roll daily, how
/// long it lasts, what direct effects it has, and what parameters
/// it carries. The [`id`](EventDef::id) doubles as the localization
/// key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDef {
    /// Unique identifier and localization key (e.g., `"surge"`, `"faction_war"`).
    pub id: Id<Event>,
    /// Base per-day roll weight. `None` means this event never
    /// rolls from the daily scheduler — it only fires when a
    /// quest `TriggerEvent` consequence spawns it directly. A
    /// `Some(w)` value is scaled by world escalation at roll
    /// time, so values around `0.05..0.8` are typical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_weight: Option<f32>,
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

fn is_false(v: &bool) -> bool {
    !*v
}
