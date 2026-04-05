use cordon_core::economy::item::ItemKind;
use cordon_core::world::event::{Event, EventKind};
use cordon_core::world::time::Day;
use rand::Rng;

use crate::state::world::World;

/// Roll for daily events. Called at the start of each day (morning phase).
pub fn roll_daily_events(world: &mut World) {
    let day = world.time.day;
    let day_num = day.0;

    // Escalation: events get more frequent as days progress
    let escalation = (day_num as f32 / 100.0).min(1.0);

    if world.rng.r#gen::<f32>() < 0.20 + escalation * 0.1 {
        if let Some(event) = roll_environmental(day, &mut world.rng) {
            world.active_events.push(event);
        }
    }

    if world.rng.r#gen::<f32>() < 0.15 + escalation * 0.05 {
        if let Some(event) = roll_economic(day, &mut world.rng) {
            world.active_events.push(event);
        }
    }

    if world.rng.r#gen::<f32>() < 0.25 + escalation * 0.1 {
        let faction_ids = world.faction_ids.clone();
        if let Some(event) = roll_faction(day, &faction_ids, &mut world.rng) {
            world.active_events.push(event);
        }
    }

    if world.rng.r#gen::<f32>() < 0.15 + escalation * 0.05 {
        let faction_ids = world.faction_ids.clone();
        if let Some(event) = roll_bunker(day, &faction_ids, &mut world.rng) {
            world.active_events.push(event);
        }
    }

    if world.rng.r#gen::<f32>() < 0.10 + escalation * 0.05 {
        if let Some(event) = roll_personal(day, &mut world.rng) {
            world.active_events.push(event);
        }
    }
}

/// Remove expired events.
pub fn expire_events(world: &mut World) {
    let day = world.time.day;
    world.active_events.retain(|e| !e.is_expired(day));
}

fn pick_faction(faction_ids: &[cordon_core::object::id::Id], rng: &mut impl Rng) -> cordon_core::object::id::Id {
    faction_ids[rng.gen_range(0..faction_ids.len())].clone()
}

fn roll_environmental(day: Day, rng: &mut impl Rng) -> Option<Event> {
    let roll: f32 = rng.r#gen();
    let (kind, duration) = if roll < 0.3 {
        (EventKind::Surge, 1)
    } else if roll < 0.45 {
        (EventKind::Blowout, 1)
    } else if roll < 0.65 {
        (EventKind::CreatureSwarm, rng.gen_range(2..=3))
    } else if roll < 0.85 {
        (EventKind::HazardShift, 30)
    } else {
        (EventKind::PsiWave, rng.gen_range(1..=2))
    };

    Some(Event { kind, duration_days: duration, day_started: day })
}

fn roll_economic(day: Day, rng: &mut impl Rng) -> Option<Event> {
    let roll: f32 = rng.r#gen();
    let (kind, duration) = if roll < 0.25 {
        (EventKind::SupplyDrop, rng.gen_range(2..=3))
    } else if roll < 0.55 {
        let shortage_kind = match rng.gen_range(0..4) {
            0 => ItemKind::Food,
            1 => ItemKind::Med,
            2 => ItemKind::Ammo,
            _ => ItemKind::Weapon,
        };
        (EventKind::Shortage(shortage_kind), rng.gen_range(3..=5))
    } else if roll < 0.75 {
        (EventKind::BlackMarketBust, rng.gen_range(2..=4))
    } else if roll < 0.9 {
        (EventKind::NewRoute, rng.gen_range(5..=15))
    } else {
        (EventKind::TraderRivalry, rng.gen_range(5..=10))
    };

    Some(Event { kind, duration_days: duration, day_started: day })
}

fn roll_faction(day: Day, faction_ids: &[cordon_core::object::id::Id], rng: &mut impl Rng) -> Option<Event> {
    let roll: f32 = rng.r#gen();
    let (kind, duration) = if roll < 0.25 {
        let a = pick_faction(faction_ids, rng);
        let mut b = pick_faction(faction_ids, rng);
        while b == a {
            b = pick_faction(faction_ids, rng);
        }
        (EventKind::FactionWar(a, b), rng.gen_range(3..=7))
    } else if roll < 0.4 {
        let a = pick_faction(faction_ids, rng);
        let mut b = pick_faction(faction_ids, rng);
        while b == a {
            b = pick_faction(faction_ids, rng);
        }
        (EventKind::FactionTruce(a, b), rng.gen_range(5..=10))
    } else if roll < 0.55 {
        (EventKind::Coup(pick_faction(faction_ids, rng)), 1)
    } else if roll < 0.7 {
        (EventKind::FactionMission(pick_faction(faction_ids, rng)), rng.gen_range(2..=5))
    } else if roll < 0.85 {
        (EventKind::FactionPatrol(pick_faction(faction_ids, rng)), 1)
    } else if roll < 0.93 {
        (EventKind::MercenaryContract, rng.gen_range(2..=3))
    } else {
        (EventKind::DevotedPilgrimage, rng.gen_range(3..=5))
    };

    Some(Event { kind, duration_days: duration, day_started: day })
}

fn roll_bunker(day: Day, faction_ids: &[cordon_core::object::id::Id], rng: &mut impl Rng) -> Option<Event> {
    let roll: f32 = rng.r#gen();
    let (kind, duration) = if roll < 0.25 {
        (EventKind::Raid(pick_faction(faction_ids, rng)), 1)
    } else if roll < 0.45 {
        (EventKind::Inspection(pick_faction(faction_ids, rng)), 1)
    } else if roll < 0.6 {
        (EventKind::PowerOutage, rng.gen_range(1..=2))
    } else if roll < 0.75 {
        (EventKind::Visitor, 1)
    } else if roll < 0.85 {
        (EventKind::Infestation, 1)
    } else if roll < 0.93 {
        (EventKind::Sabotage, 1)
    } else {
        (EventKind::BreakIn, 1)
    };

    Some(Event { kind, duration_days: duration, day_started: day })
}

fn roll_personal(day: Day, rng: &mut impl Rng) -> Option<Event> {
    let roll: f32 = rng.r#gen();
    let (kind, duration) = if roll < 0.3 {
        (EventKind::DebtCollector, rng.gen_range(2..=5))
    } else if roll < 0.6 {
        (EventKind::WoundedStranger, 1)
    } else {
        (EventKind::OldFriend, rng.gen_range(1..=3))
    };

    Some(Event { kind, duration_days: duration, day_started: day })
}
