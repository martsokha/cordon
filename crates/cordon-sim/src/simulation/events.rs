//! Event scheduling and resolution.
//!
//! Events are data-driven: [`EventDef`]s from config define what can
//! happen, their probabilities, and durations. The sim rolls daily
//! for each eligible event and creates [`ActiveEvent`] instances.

use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::event::{ActiveEvent, EventCategory, EventDef};
use rand::{Rng, RngExt};

use crate::state::world::World;

/// Roll for daily events using loaded event definitions.
///
/// For each event def, checks eligibility (earliest day, stackability)
/// then rolls against its base probability modified by escalation and
/// world state. Creates [`ActiveEvent`] instances for events that fire.
pub fn roll_daily_events(world: &mut World, event_defs: &[EventDef]) {
    let day = world.time.day;
    let day_num = day.value();

    // Escalation: events get more frequent as days progress
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

    for def in event_defs {
        // Check earliest day
        if day < def.earliest_day {
            continue;
        }

        // Check max instances
        if let Some(max) = def.max_instances {
            let active_count = world
                .active_events
                .iter()
                .filter(|e| e.def_id == def.id)
                .count();
            if active_count >= max as usize {
                continue;
            }
        }

        // Roll probability
        let probability = def.base_probability * category_multiplier(def.category);
        if world.rng.events.random::<f32>() >= probability {
            continue;
        }

        // Roll duration
        let duration = if def.min_duration == def.max_duration {
            def.min_duration
        } else {
            world
                .rng
                .events
                .random_range(def.min_duration..=def.max_duration)
        };

        // Pick involved factions (if the def specifies candidates)
        let involved_factions = pick_involved_factions(
            &def.involved_factions,
            &world.faction_ids,
            &mut world.rng.events,
        );

        // Pick target sector (if the def specifies candidates)
        let target_area = if def.target_areas.is_empty() {
            None
        } else {
            let idx = world.rng.events.random_range(0..def.target_areas.len());
            Some(def.target_areas[idx].clone())
        };

        world.active_events.push(ActiveEvent {
            def_id: def.id.clone(),
            day_started: day,
            duration_days: duration,
            involved_factions,
            target_area,
        });
    }
}

/// Remove expired events.
pub fn expire_events(world: &mut World) {
    let day = world.time.day;
    world.active_events.retain(|e| !e.is_expired(day));
}

/// Pick faction IDs for an event instance.
///
/// If the def lists specific factions, picks from those.
/// Otherwise picks from all world factions.
fn pick_involved_factions(
    def_factions: &[Id<Faction>],
    world_factions: &[Id<Faction>],
    rng: &mut impl Rng,
) -> Vec<Id<Faction>> {
    let pool = if def_factions.is_empty() {
        world_factions
    } else {
        def_factions
    };

    if pool.is_empty() {
        return Vec::new();
    }

    // Most events involve 0-2 factions. Pick up to 2 unique ones.
    let mut result = Vec::new();
    if !pool.is_empty() {
        let a = pool[rng.random_range(0..pool.len())].clone();
        result.push(a.clone());

        if pool.len() > 1 {
            let mut b = pool[rng.random_range(0..pool.len())].clone();
            while b == a {
                b = pool[rng.random_range(0..pool.len())].clone();
            }
            result.push(b);
        }
    }

    result
}
