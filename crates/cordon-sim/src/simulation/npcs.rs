use cordon_core::entity::npc::{Npc, NpcCondition, NpcType, Need, Personality};
use rand::Rng;

use crate::state::world::World;

/// Generate the day's visitors. Called during morning phase.
pub fn spawn_daily_visitors(world: &mut World) -> Vec<Npc> {
    let mut visitors = Vec::new();
    let day = world.time.day.0;

    let base_count = 3 + (day / 10).min(5);
    let count = world.rng.gen_range(base_count..=base_count + 3);

    for _ in 0..count {
        let npc = generate_visitor(world);
        visitors.push(npc);
    }

    visitors
}

fn generate_visitor(world: &mut World) -> Npc {
    let id = world.alloc_uid();
    let faction = world.random_faction();
    let rank = pick_rank(&mut world.rng);
    let npc_type = pick_npc_type(&mut world.rng);
    let personality = pick_personality(&mut world.rng);
    let wealth = generate_wealth(rank, &mut world.rng);

    Npc {
        id,
        name: generate_name(&mut world.rng),
        faction,
        rank,
        npc_type,
        gear: Vec::new(), // TODO: generate gear based on faction/rank from config
        condition: NpcCondition::Healthy,
        trust: 0.0,
        wealth,
        need: Need::None,
        personality,
        perks: Vec::new(), // TODO: pick from perk config
        revealed_perks: Vec::new(),
        role: None,
        loyalty: 0.5,
        daily_pay: rank_pay_base(rank),
    }
}

fn pick_rank(rng: &mut impl Rng) -> u8 {
    let roll: f32 = rng.r#gen();
    if roll < 0.4 { 1 }
    else if roll < 0.7 { 2 }
    else if roll < 0.9 { 3 }
    else if roll < 0.97 { 4 }
    else { 5 }
}

fn pick_npc_type(rng: &mut impl Rng) -> NpcType {
    let roll: f32 = rng.r#gen();
    if roll < 0.5 { NpcType::Drifter }
    else if roll < 0.7 { NpcType::FactionSoldier }
    else if roll < 0.8 { NpcType::JobSeeker }
    else if roll < 0.88 { NpcType::Scammer }
    else if roll < 0.93 { NpcType::DesperateVisitor }
    else if roll < 0.97 { NpcType::Informant }
    else { NpcType::FactionRep }
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

fn generate_wealth(rank: u8, rng: &mut impl Rng) -> u32 {
    let base: u32 = match rank {
        1 => 200,
        2 => 800,
        3 => 2000,
        4 => 5000,
        _ => 15000,
    };
    let jitter = rng.gen_range(0.5_f32..1.5);
    (base as f32 * jitter) as u32
}

fn rank_pay_base(rank: u8) -> u32 {
    match rank {
        1 => 100,
        2 => 150,
        3 => 250,
        4 => 400,
        _ => 700,
    }
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
