use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::{NameFormat, NamePool, NpcName};
use cordon_core::entity::npc::{Npc, Personality};
use cordon_core::item::Inventory;
use cordon_core::primitive::credits::Credits;
use cordon_core::primitive::experience::Experience;
use cordon_core::primitive::id::Id;
use cordon_core::primitive::uid::Uid;
use rand::{Rng, RngExt};

use crate::state::world::World;

/// Generates NPC attributes. Each method produces one aspect of an NPC.
///
/// The default implementation provides reasonable random distributions.
/// Override individual methods to customize generation for specific
/// factions, game phases, or testing.
pub trait NpcGenerator {
    /// How many visitors arrive today.
    fn visitor_count<R: Rng>(&self, day: u32, rng: &mut R) -> u32 {
        let base_count = 30 + (day / 5).min(20);
        rng.random_range(base_count..=base_count + 10)
    }

    /// Pick name keys from a name pool.
    fn generate_name<R: Rng>(&self, pool: &NamePool, rng: &mut R) -> NpcName {
        let mut pick = |list: &[String]| -> String {
            if list.is_empty() {
                "unknown".to_string()
            } else {
                list[rng.random_range(0..list.len())].clone()
            }
        };

        match pool.format {
            NameFormat::Alias => NpcName {
                format: NameFormat::Alias,
                first: pick(&pool.aliases),
                second: None,
            },
            NameFormat::FirstSurname => NpcName {
                format: NameFormat::FirstSurname,
                first: pick(&pool.names),
                second: Some(pick(&pool.surnames)),
            },
            NameFormat::FirstAlias => NpcName {
                format: NameFormat::FirstAlias,
                first: pick(&pool.names),
                second: Some(pick(&pool.aliases)),
            },
        }
    }

    /// Generate experience for a visiting NPC. Weighted toward low ranks.
    fn generate_xp<R: Rng>(&self, rng: &mut R) -> Experience {
        let roll: f32 = rng.random::<f32>();
        let xp = if roll < 0.4 {
            rng.random_range(0..100)
        } else if roll < 0.7 {
            rng.random_range(100..500)
        } else if roll < 0.9 {
            rng.random_range(500..2000)
        } else if roll < 0.97 {
            rng.random_range(2000..10000)
        } else {
            rng.random_range(10000..30000)
        };
        Experience::new(xp)
    }

    /// Generate a personality trait.
    fn generate_personality<R: Rng>(&self, rng: &mut R) -> Personality {
        let options = [
            Personality::Cautious,
            Personality::Aggressive,
            Personality::Honest,
            Personality::Deceptive,
            Personality::Patient,
            Personality::Impulsive,
        ];
        options[rng.random_range(0..options.len())]
    }

    /// Generate wealth based on rank tier.
    fn generate_wealth<R: Rng>(&self, rank: u8, rng: &mut R) -> Credits {
        let base: u32 = match rank {
            1 => 200,
            2 => 800,
            3 => 2000,
            4 => 5000,
            _ => 15000,
        };
        let jitter = rng.random_range(0.5_f32..1.5);
        Credits::new((base as f32 * jitter) as u32)
    }

    /// Base daily pay for an employed NPC of a given rank.
    fn daily_pay(&self, rank: u8) -> Credits {
        match rank {
            1 => Credits::new(100),
            2 => Credits::new(150),
            3 => Credits::new(250),
            4 => Credits::new(400),
            _ => Credits::new(700),
        }
    }

    /// Inventory slot count based on rank.
    fn inventory_slots(&self, rank: u8) -> u8 {
        8 + rank * 2
    }

    /// Build a complete NPC from generated parts.
    fn generate<R: Rng>(
        &self,
        id: Uid<Npc>,
        faction: Id<Faction>,
        name_pool: &NamePool,
        rng: &mut R,
    ) -> Npc {
        let xp = self.generate_xp(rng);
        let rank = xp.npc_rank();
        let personality = self.generate_personality(rng);
        let wealth = self.generate_wealth(rank, rng);

        Npc {
            id,
            name: self.generate_name(name_pool, rng),
            faction,
            xp,
            inventory: Inventory::new(self.inventory_slots(rank)),
            health: cordon_core::primitive::health::Health::FULL,
            trust: 0.0,
            wealth,
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
    let day = world.time.day.value();
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
