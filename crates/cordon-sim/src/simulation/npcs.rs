use cordon_core::entity::name::{NameFormat, NamePool};
use cordon_core::entity::npc::{Need, Npc, NpcCondition, Personality, npc_rank_from_xp};
use cordon_core::primitive::experience::Experience;
use cordon_core::primitive::id::{Faction, Id};
use cordon_core::primitive::uid::Uid;
use rand::Rng;
use rand::rngs::StdRng;

use crate::state::world::World;

/// Generates NPC attributes. Each method produces one aspect of an NPC.
///
/// The default implementation provides reasonable random distributions.
/// Override individual methods to customize generation for specific
/// factions, game phases, or testing.
pub trait NpcGenerator {
    /// How many visitors arrive today.
    fn visitor_count(&self, day: u32, rng: &mut StdRng) -> u32 {
        let base_count = 3 + (day / 10).min(5);
        rng.gen_range(base_count..=base_count + 3)
    }

    /// Generate a display name from a name pool.
    fn generate_name(&self, pool: &NamePool, rng: &mut StdRng) -> String {
        match pool.format {
            NameFormat::Alias => {
                if pool.names.is_empty() {
                    return "Unknown".to_string();
                }
                pool.names[rng.gen_range(0..pool.names.len())].clone()
            }
            NameFormat::FirstSurname => {
                let first = if pool.names.is_empty() {
                    "Unknown"
                } else {
                    &pool.names[rng.gen_range(0..pool.names.len())]
                };
                let last = if pool.surnames.is_empty() {
                    ""
                } else {
                    &pool.surnames[rng.gen_range(0..pool.surnames.len())]
                };
                if last.is_empty() {
                    first.to_string()
                } else {
                    format!("{first} {last}")
                }
            }
            NameFormat::TitleName => {
                let title = if pool.titles.is_empty() {
                    "Brother"
                } else {
                    &pool.titles[rng.gen_range(0..pool.titles.len())]
                };
                let name = if pool.names.is_empty() {
                    "Unknown"
                } else {
                    &pool.names[rng.gen_range(0..pool.names.len())]
                };
                format!("{title} {name}")
            }
        }
    }

    /// Generate experience for a visiting NPC. Weighted toward low ranks.
    fn generate_xp(&self, rng: &mut StdRng) -> Experience {
        let roll: f32 = rng.r#gen();
        let xp = if roll < 0.4 {
            rng.gen_range(0..100)
        } else if roll < 0.7 {
            rng.gen_range(100..500)
        } else if roll < 0.9 {
            rng.gen_range(500..2000)
        } else if roll < 0.97 {
            rng.gen_range(2000..10000)
        } else {
            rng.gen_range(10000..30000)
        };
        Experience::new(xp)
    }

    /// Generate a personality trait.
    fn generate_personality(&self, rng: &mut StdRng) -> Personality {
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

    /// Generate wealth based on rank tier.
    fn generate_wealth(&self, rank: u8, rng: &mut StdRng) -> u32 {
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

    /// Base daily pay for an employed NPC of a given rank.
    fn daily_pay(&self, rank: u8) -> u32 {
        match rank {
            1 => 100,
            2 => 150,
            3 => 250,
            4 => 400,
            _ => 700,
        }
    }

    /// Inventory slot count based on rank.
    fn inventory_slots(&self, rank: u8) -> u8 {
        8 + rank * 2
    }

    /// Build a complete NPC from generated parts.
    fn generate(
        &self,
        id: Uid,
        faction: Id<Faction>,
        name_pool: &NamePool,
        rng: &mut StdRng,
    ) -> Npc {
        let xp = self.generate_xp(rng);
        let rank = npc_rank_from_xp(xp);
        let personality = self.generate_personality(rng);
        let wealth = self.generate_wealth(rank, rng);

        Npc {
            id,
            name: self.generate_name(name_pool, rng),
            faction,
            xp,
            gear: Vec::new(),
            condition: NpcCondition::Healthy,
            inventory_slots: self.inventory_slots(rank),
            trust: 0.0,
            wealth,
            need: Need::None,
            personality,
            perks: Vec::new(),
            revealed_perks: Vec::new(),
            role: None,
            loyalty: 0.5,
            daily_pay: self.daily_pay(rank),
        }
    }
}

/// Default NPC generator with standard random distributions.
pub struct DefaultNpcGenerator;

impl NpcGenerator for DefaultNpcGenerator {}

/// Resolve a faction's name pool, with a fallback for missing pools.
pub fn resolve_name_pool<'a>(
    faction: &Id<Faction>,
    name_pools: &'a std::collections::HashMap<Id<Faction>, NamePool>,
    fallback: &'a NamePool,
) -> &'a NamePool {
    name_pools.get(faction).unwrap_or(fallback)
}

/// Generate the day's visitors.
///
/// `name_pools` maps faction IDs directly to their name pools
/// (pre-resolved from GameData at startup).
pub fn spawn_daily_visitors(
    world: &mut World,
    generator: &impl NpcGenerator,
    name_pools: &std::collections::HashMap<Id<Faction>, NamePool>,
    fallback_pool: &NamePool,
) -> Vec<Npc> {
    let day = world.time.day.0;
    let count = generator.visitor_count(day, &mut world.rng.npcs);

    let mut visitors = Vec::new();
    for _ in 0..count {
        let id = world.alloc_uid();
        let faction = world.random_faction();
        let pool = resolve_name_pool(&faction, name_pools, fallback_pool);
        let npc = generator.generate(id, faction, pool, &mut world.rng.npcs);
        visitors.push(npc);
    }

    visitors
}
