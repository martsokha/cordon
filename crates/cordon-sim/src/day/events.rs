//! Event scheduling and resolution.
//!
//! Pure functions over `Vec<ActiveEvent>` and the loaded `EventDef`
//! catalog. Called from the day-cycle systems in [`crate::day`]
//! (daily probability rolls) and the quest consequence applier
//! (on-demand instantiation via [`spawn_event_instance`]).

use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{Day, Id};
use cordon_core::world::event::{ActiveEvent, EventCategory, EventDef};
use rand::{Rng, RngExt};

/// Roll for daily events using loaded event definitions.
///
/// For each event def, checks eligibility (earliest day, stack cap),
/// then rolls against its base probability modified by escalation and
/// world state. Pushes any successful rolls onto `active`.
pub fn roll_daily_events<R: Rng>(
    active: &mut Vec<ActiveEvent>,
    defs: &[EventDef],
    faction_ids: &[Id<Faction>],
    day: Day,
    rng: &mut R,
) {
    let day_num = day.value();
    // Escalation: events get more frequent as days progress.
    let escalation = (day_num as f32 / 100.0).min(1.0);

    let category_multiplier = |cat: EventCategory| -> f32 {
        match cat {
            EventCategory::Environmental => 1.0 + escalation * 0.5,
            EventCategory::Economic => 1.0 + escalation * 0.3,
            EventCategory::Faction => 1.0 + escalation * 0.4,
            EventCategory::Bunker => 1.0 + escalation * 0.3,
            EventCategory::Personal => 1.0 + escalation * 0.2,
        }
    };

    for def in defs {
        if day < def.earliest_day {
            continue;
        }
        if let Some(max) = def.max_instances {
            let active_count = active.iter().filter(|e| e.def_id == def.id).count();
            if active_count >= max as usize {
                continue;
            }
        }

        let probability = def.base_probability * category_multiplier(def.category);
        if rng.random::<f32>() >= probability {
            continue;
        }

        active.push(spawn_event_instance(def, faction_ids, day, rng));
    }
}

/// Build a fresh [`ActiveEvent`] from a definition, rolling a
/// random duration in `[min_duration, max_duration]`, picking
/// the involved factions from the def's list (or from the
/// world list if empty), and picking a target area from the
/// def's list (or `None` for zone-wide events).
///
/// Used both by [`roll_daily_events`] (after its probability
/// roll succeeds) and by the quest `TriggerEvent` consequence
/// applier (which bypasses probability but still wants
/// randomness on the instance parameters). Keeping both sites
/// on the same helper means the two paths cannot drift.
pub fn spawn_event_instance<R: Rng>(
    def: &EventDef,
    faction_ids: &[Id<Faction>],
    day: Day,
    rng: &mut R,
) -> ActiveEvent {
    let duration = if def.min_duration == def.max_duration {
        def.min_duration
    } else {
        rng.random_range(def.min_duration..=def.max_duration)
    };

    let involved_factions = pick_involved_factions(&def.involved_factions, faction_ids, rng);

    let target_area = if def.target_areas.is_empty() {
        None
    } else {
        let idx = rng.random_range(0..def.target_areas.len());
        Some(def.target_areas[idx].clone())
    };

    ActiveEvent {
        def_id: def.id.clone(),
        day_started: day,
        duration_days: duration,
        involved_factions,
        target_area,
    }
}

/// Drop expired events from the active log.
pub fn expire_events(active: &mut Vec<ActiveEvent>, day: Day) {
    active.retain(|e| !e.is_expired(day));
}

/// Pick faction IDs for an event instance.
///
/// If the def lists specific factions, picks from those; otherwise
/// picks from all loaded factions. At most two unique factions per
/// event.
fn pick_involved_factions<R: Rng>(
    def_factions: &[Id<Faction>],
    world_factions: &[Id<Faction>],
    rng: &mut R,
) -> Vec<Id<Faction>> {
    let pool = if def_factions.is_empty() {
        world_factions
    } else {
        def_factions
    };

    if pool.is_empty() {
        return Vec::new();
    }

    let a = pool[rng.random_range(0..pool.len())].clone();
    let mut result = vec![a.clone()];
    if pool.len() > 1 {
        let mut b = pool[rng.random_range(0..pool.len())].clone();
        while b == a {
            b = pool[rng.random_range(0..pool.len())].clone();
        }
        result.push(b);
    }
    result
}
