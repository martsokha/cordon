use std::collections::HashMap;

use cordon_core::entity::archetype::{Archetype, ArchetypeDef, SquadGoalKind, SquadTemplate};
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::{NameFormat, NamePool, NpcName};
use cordon_core::entity::npc::{Npc, Personality};
use cordon_core::entity::squad::{Goal, Squad};
use cordon_core::item::{Item, ItemDef, Loadout};
use cordon_core::primitive::{Credits, Experience, Health, Id, Rank, Uid};
use cordon_core::world::area::{Area, AreaDef};
use rand::{Rng, RngExt};

use crate::resources::{FactionIndex, UidAllocator};
use crate::world::loadout::generate_loadout;

/// Read-only references the loadout generator needs at NPC spawn time.
pub struct LoadoutContext<'a> {
    pub archetypes: &'a HashMap<Id<Archetype>, ArchetypeDef>,
    pub items: &'a HashMap<Id<Item>, ItemDef>,
    pub areas: &'a HashMap<Id<Area>, AreaDef>,
}

/// One day's worth of fresh NPCs and squads, ready for the game layer
/// to insert into the world and spawn ECS entities for.
pub struct DailySpawn {
    pub npcs: Vec<Npc>,
    pub squads: Vec<Squad>,
}

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

    /// Generate wealth based on rank.
    fn generate_wealth<R: Rng>(&self, rank: Rank, rng: &mut R) -> Credits {
        let base: u32 = match rank {
            Rank::Novice => 200,
            Rank::Experienced => 800,
            Rank::Veteran => 2000,
            Rank::Master => 5000,
            Rank::Legend => 15000,
        };
        let jitter = rng.random_range(0.5_f32..1.5);
        Credits::new((base as f32 * jitter) as u32)
    }

    /// Base daily pay for an employed NPC of a given rank.
    fn daily_pay(&self, rank: Rank) -> Credits {
        Credits::new(match rank {
            Rank::Novice => 100,
            Rank::Experienced => 150,
            Rank::Veteran => 250,
            Rank::Master => 400,
            Rank::Legend => 700,
        })
    }

    /// Build a complete NPC from generated parts. The caller passes in
    /// the desired rank explicitly so squad spawning can produce
    /// rank-correct members from a template.
    fn generate_with_rank<R: Rng>(
        &self,
        id: Uid<Npc>,
        faction: Id<Faction>,
        rank: Rank,
        name_pool: &NamePool,
        rng: &mut R,
    ) -> Npc {
        let xp = Experience::new(rank.xp_threshold());
        let personality = self.generate_personality(rng);
        let wealth = self.generate_wealth(rank, rng);

        Npc {
            id,
            name: self.generate_name(name_pool, rng),
            faction,
            xp,
            loadout: Loadout::new(),
            health: Health::FULL,
            max_hp: 100,
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

    /// Target alive-NPC population in the Zone. Spawning replenishes
    /// toward this number rather than dumping a fixed batch each day.
    fn target_population(&self, _day: u32) -> u32 {
        1000
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

/// Roll fresh NPCs and squads to top up `deficit` members.
///
/// Caller is responsible for counting the alive population (e.g. via
/// a Bevy query) and computing the deficit. This function only does
/// the rolling and returns the produced data; the caller spawns
/// entities from the returned [`DailySpawn`].
pub fn roll_population_top_up<R: Rng>(
    rng: &mut R,
    uids: &mut UidAllocator,
    factions: &FactionIndex,
    generator: &impl NpcGenerator,
    name_pools: &HashMap<Id<Faction>, NamePool>,
    fallback_pool: &NamePool,
    loadout_ctx: &LoadoutContext<'_>,
    area_ids: &[Id<Area>],
    deficit: u32,
) -> DailySpawn {
    let mut deficit = deficit;
    let mut spawn = DailySpawn {
        npcs: Vec::new(),
        squads: Vec::new(),
    };

    if factions.0.is_empty() {
        return spawn;
    }

    // Spawn squads until the deficit is filled. Each squad consumes
    // `template.ranks.len()` from the deficit. A safety counter caps
    // attempts so a misconfigured archetype catalog can't spin forever.
    let mut attempts_remaining = (deficit as usize) * 4 + 16;
    while deficit > 0 && attempts_remaining > 0 {
        attempts_remaining -= 1;
        let faction = factions.0[rng.random_range(0..factions.0.len())].clone();
        let arch = loadout_ctx
            .archetypes
            .get(&Id::<Archetype>::new(faction.as_str()));
        let Some(arch) = arch else { continue };

        // Pick a squad template from this faction's pool.
        let Some(template) = pick_squad_template(&arch.squads, rng) else {
            continue;
        };

        // Roll the members listed in the template.
        let pool = resolve_name_pool(&faction, name_pools, fallback_pool);
        let mut member_uids: Vec<Uid<Npc>> = Vec::with_capacity(template.ranks.len());
        let mut leader_uid: Option<Uid<Npc>> = None;
        let mut highest_rank = Rank::Novice;

        for (slot_idx, rank) in template.ranks.iter().enumerate() {
            let npc_uid = uids.alloc::<Npc>();
            let mut npc =
                generator.generate_with_rank(npc_uid, faction.clone(), *rank, pool, rng);
            // Roll a loadout for this member from the per-rank pool.
            npc.loadout = generate_loadout(arch, npc.rank(), loadout_ctx.items, rng);

            member_uids.push(npc_uid);
            if slot_idx == 0 || *rank > highest_rank {
                leader_uid = Some(npc_uid);
                highest_rank = *rank;
            }
            spawn.npcs.push(npc);
        }

        let Some(leader) = leader_uid else { continue };

        // Resolve the template's coarse goal kind into a concrete Goal.
        let goal = resolve_goal(template.goal, area_ids, rng);

        // For Patrol/Scavenge goals, scatter 3 waypoints inside the
        // target area so multiple squads patrolling the same area don't
        // converge on a single point.
        let waypoints = waypoints_for_goal(&goal, loadout_ctx, rng);

        let squad_uid = uids.alloc::<Squad>();
        let member_count = template.ranks.len() as u32;
        let squad = Squad {
            id: squad_uid,
            faction: faction.clone(),
            members: member_uids,
            leader,
            goal,
            formation: template.formation,
            facing: [0.0, 1.0],
            waypoints,
            next_waypoint: 0,
        };
        spawn.squads.push(squad);
        deficit = deficit.saturating_sub(member_count);
    }

    spawn
}

/// Roll one squad template from a weighted pool. Returns `None` if the
/// pool is empty.
fn pick_squad_template<'a, R: Rng>(
    pool: &'a [SquadTemplate],
    rng: &mut R,
) -> Option<&'a SquadTemplate> {
    if pool.is_empty() {
        return None;
    }
    let total: u32 = pool.iter().map(|t| t.weight.max(1)).sum();
    if total == 0 {
        return Some(&pool[0]);
    }
    let mut roll = rng.random_range(0..total);
    for entry in pool {
        let w = entry.weight.max(1);
        if roll < w {
            return Some(entry);
        }
        roll -= w;
    }
    pool.last()
}

/// Translate a template's goal kind into a concrete [`Goal`] by picking
/// a target area when needed.
fn resolve_goal<R: Rng>(kind: SquadGoalKind, area_ids: &[Id<Area>], rng: &mut R) -> Goal {
    fn pick_area<R: Rng>(area_ids: &[Id<Area>], rng: &mut R) -> Option<Id<Area>> {
        if area_ids.is_empty() {
            None
        } else {
            Some(area_ids[rng.random_range(0..area_ids.len())].clone())
        }
    }
    match kind {
        SquadGoalKind::Idle => Goal::Idle,
        SquadGoalKind::Patrol => pick_area(area_ids, rng)
            .map(|area| Goal::Patrol { area })
            .unwrap_or(Goal::Idle),
        SquadGoalKind::Scavenge => pick_area(area_ids, rng)
            .map(|area| Goal::Scavenge { area })
            .unwrap_or(Goal::Idle),
    }
}

/// Roll 3 random waypoints inside the goal's area, scattered around
/// the area centre at varying angles. Empty for non-area goals.
fn waypoints_for_goal<R: Rng>(
    goal: &Goal,
    ctx: &LoadoutContext<'_>,
    rng: &mut R,
) -> Vec<[f32; 2]> {
    let area_id = match goal {
        Goal::Patrol { area } | Goal::Scavenge { area } => area,
        _ => return Vec::new(),
    };
    let Some(area) = ctx.areas.get(area_id) else {
        return Vec::new();
    };
    let cx = area.location.x;
    let cy = area.location.y;
    let r = area.radius.value() * 0.7; // Stay inside the visible disk.
    (0..3)
        .map(|_| {
            let angle = rng.random_range(0.0_f32..std::f32::consts::TAU);
            let dist = rng.random_range(r * 0.3..r);
            [cx + angle.cos() * dist, cy + angle.sin() * dist]
        })
        .collect()
}
