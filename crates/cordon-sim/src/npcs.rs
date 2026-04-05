use cordon_core::entity::faction::FactionId;
use cordon_core::entity::npc::{Npc, NpcCondition, NpcType, Need, Personality, Rank};
use rand::Rng;

use crate::world::World;

/// Generate the day's visitors. Called during morning phase.
pub fn spawn_daily_visitors(world: &mut World) -> Vec<Npc> {
    let mut visitors = Vec::new();
    let day = world.time.day.0;

    // Base visitor count scales slightly with time
    let base_count = 3 + (day / 10).min(5);
    let count = world.rng.gen_range(base_count..=base_count + 3);

    for _ in 0..count {
        let npc = generate_visitor(world);
        visitors.push(npc);
    }

    visitors
}

fn generate_visitor(world: &mut World) -> Npc {
    let id = world.alloc_npc_id();

    // Weight factions by player standing and world state
    let faction = pick_faction(&mut world.rng);
    let rank = pick_rank(&mut world.rng);
    let npc_type = pick_npc_type(&mut world.rng);
    let personality = pick_personality(&mut world.rng);

    Npc {
        id,
        name: generate_name(&mut world.rng),
        faction,
        rank,
        npc_type,
        gear: Vec::new(), // TODO: generate gear based on faction/rank
        condition: NpcCondition::Healthy,
        trust: 0.0,
        wealth: generate_wealth(rank, faction, &mut world.rng),
        need: Need::None,
        personality,
        perks: generate_perks(&mut world.rng),
        revealed_perks: Vec::new(),
        role: None,
        loyalty: 0.5,
        daily_pay: base_pay(faction, rank),
    }
}

fn pick_faction(rng: &mut impl Rng) -> FactionId {
    // Drifters are most common, then faction soldiers, etc.
    let weights = [
        (FactionId::Drifters, 35),
        (FactionId::Order, 10),
        (FactionId::Collective, 10),
        (FactionId::Syndicate, 12),
        (FactionId::Garrison, 10),
        (FactionId::Institute, 5),
        (FactionId::Mercenaries, 10),
        (FactionId::Devoted, 8),
    ];

    let total: u32 = weights.iter().map(|(_, w)| w).sum();
    let mut roll = rng.gen_range(0..total);

    for (faction, weight) in &weights {
        if roll < *weight {
            return *faction;
        }
        roll -= weight;
    }

    FactionId::Drifters
}

fn pick_rank(rng: &mut impl Rng) -> Rank {
    let roll: f32 = rng.r#gen();
    if roll < 0.4 {
        Rank::Tier1
    } else if roll < 0.7 {
        Rank::Tier2
    } else if roll < 0.9 {
        Rank::Tier3
    } else if roll < 0.97 {
        Rank::Tier4
    } else {
        Rank::Tier5
    }
}

fn pick_npc_type(rng: &mut impl Rng) -> NpcType {
    let roll: f32 = rng.r#gen();
    if roll < 0.5 {
        NpcType::Drifter
    } else if roll < 0.7 {
        NpcType::FactionSoldier
    } else if roll < 0.8 {
        NpcType::JobSeeker
    } else if roll < 0.88 {
        NpcType::Scammer
    } else if roll < 0.93 {
        NpcType::DesperateVisitor
    } else if roll < 0.97 {
        NpcType::Informant
    } else {
        NpcType::FactionRep
    }
}

fn pick_personality(rng: &mut impl Rng) -> Personality {
    let options = [
        Personality::Cautious,
        Personality::Aggressive,
        Personality::Honest,
        Personality::Deceptive,
        Personality::Patient,
        Personality::Impulsive,
    ];
    options[rng.gen_range(0..options.len())]
}

fn generate_perks(rng: &mut impl Rng) -> Vec<cordon_core::entity::npc::Perk> {
    use cordon_core::entity::npc::Perk;

    let all_perks = [
        Perk::ScavengersEye,
        Perk::HardToKill,
        Perk::Pathfinder,
        Perk::Haggler,
        Perk::Ghost,
        Perk::Ironwall,
        Perk::Intimidating,
        Perk::StickyFingers,
        Perk::Coward,
        Perk::BigMouth,
        Perk::GrudgeHolder,
        Perk::Lucky,
    ];

    // 1-3 perks per NPC
    let count = rng.gen_range(1..=3);
    let mut perks = Vec::new();

    for _ in 0..count {
        let perk = all_perks[rng.gen_range(0..all_perks.len())];
        if !perks.contains(&perk) {
            perks.push(perk);
        }
    }

    perks
}

fn generate_wealth(rank: Rank, faction: FactionId, rng: &mut impl Rng) -> u32 {
    let base = match rank {
        Rank::Tier1 => 200,
        Rank::Tier2 => 800,
        Rank::Tier3 => 2000,
        Rank::Tier4 => 5000,
        Rank::Tier5 => 15000,
    };

    let faction_mult = match faction {
        FactionId::Mercenaries => 2.0,
        FactionId::Garrison => 1.5,
        FactionId::Order | FactionId::Institute => 1.2,
        FactionId::Drifters => 0.6,
        FactionId::Syndicate => 0.8,
        _ => 1.0,
    };

    let jitter = rng.gen_range(0.5..1.5);
    (base as f32 * faction_mult * jitter) as u32
}

fn base_pay(faction: FactionId, rank: Rank) -> u32 {
    let rank_mult = match rank {
        Rank::Tier1 => 1.0,
        Rank::Tier2 => 1.5,
        Rank::Tier3 => 2.5,
        Rank::Tier4 => 4.0,
        Rank::Tier5 => 7.0,
    };

    let faction_base: f32 = match faction {
        FactionId::Drifters => 100.0,
        FactionId::Syndicate => 150.0,
        FactionId::Mercenaries => 300.0,
        _ => 200.0,
    };

    (faction_base * rank_mult) as u32
}

fn generate_name(rng: &mut impl Rng) -> String {
    let names = [
        "Viper", "Matches", "Ghost", "Shaggy", "Brick", "Needle", "Crow",
        "Ash", "Sparks", "Mole", "Sledge", "Fang", "Gravel", "Patch",
        "Wire", "Bolt", "Stump", "Raven", "Flint", "Scar", "Haze",
        "Copper", "Dusk", "Thorn", "Ember", "Frost", "Hex", "Pike",
        "Rust", "Slate", "Splint", "Gauge", "Knot", "Grit", "Cinder",
    ];
    names[rng.gen_range(0..names.len())].to_string()
}
