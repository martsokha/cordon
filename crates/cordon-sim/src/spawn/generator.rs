use std::collections::HashMap;

use cordon_core::entity::archetype::{Archetype, ArchetypeDef, SquadGoalKind, SquadTemplate};
use cordon_core::entity::faction::Faction;
use cordon_core::entity::name::{NameFormat, NamePool, NpcName};
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::{Goal, Squad};
use cordon_core::item::{Item, ItemDef, Loadout};
use cordon_core::primitive::{
    Corruption, Credits, Experience, Health, Id, Loyalty, Pool, Rank, Stamina, Trust, Uid,
};
use cordon_core::world::area::{Area, AreaDef};
use rand::{Rng, RngExt};

use crate::entity::npc::{
    ActiveEffects, BaseMaxes, FactionId, NpcAttributes, NpcBundle, NpcMarker,
};
use crate::resources::{FactionIndex, UidAllocator};
use crate::spawn::loadout::generate_loadout;
use crate::spawn::waypoints::roll_area_waypoints;

/// Read-only references the loadout generator needs at NPC spawn time.
pub struct LoadoutContext<'a> {
    pub archetypes: &'a HashMap<Id<Archetype>, ArchetypeDef>,
    pub items: &'a HashMap<Id<Item>, ItemDef>,
    pub areas: &'a HashMap<Id<Area>, AreaDef>,
}

/// One day's worth of fresh NPCs and squads, ready for the
/// game layer to insert into the world and spawn ECS entities
/// for. NPCs come pre-assembled as [`NpcBundle`]s so the caller
/// can `commands.spawn(bundle)` directly.
pub struct DailySpawn {
    pub npcs: Vec<NpcBundle>,
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

    /// Build a complete NPC bundle from generated parts. The
    /// caller passes in the desired rank explicitly so squad
    /// spawning can produce rank-correct members from a template.
    fn generate_with_rank<R: Rng>(
        &self,
        id: Uid<Npc>,
        faction: Id<Faction>,
        rank: Rank,
        name_pool: &NamePool,
        rng: &mut R,
    ) -> NpcBundle {
        let xp = Experience::new(rank.xp_threshold());
        let wealth = self.generate_wealth(rank, rng);
        let health: Pool<Health> = Pool::full();
        let hp_max = health.max();

        NpcBundle {
            marker: NpcMarker,
            id,
            name: self.generate_name(name_pool, rng),
            faction: FactionId(faction),
            xp,
            hp: health,
            stamina: Pool::<Stamina>::full(),
            corruption: Pool::<Corruption>::empty(),
            active_effects: ActiveEffects::default(),
            base_maxes: BaseMaxes {
                hp: hp_max,
                stamina: 100,
            },
            loadout: Loadout::new(),
            wealth,
            attributes: NpcAttributes {
                trust: Trust(0.0),
                loyalty: Loyalty(0.5),
            },
        }
    }

    /// Target alive-NPC population in the Zone. Spawning replenishes
    /// toward this number rather than dumping a fixed batch each day.
    fn target_population(&self, _day: u32) -> u32 {
        200
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
    let total_weight: u32 = factions.0.iter().map(|(_, w)| (*w).max(1)).sum();
    let mut attempts_remaining = (deficit as usize) * 4 + 16;
    while deficit > 0 && attempts_remaining > 0 {
        attempts_remaining -= 1;
        let faction = pick_weighted_faction(&factions.0, total_weight, rng);
        // Walk archetypes by faction field, not by faction id
        // lookup: the HashMap is keyed by archetype id
        // (`archetype_garrison`) which differs from the faction
        // id string after the category-prefix rename.
        let arch = loadout_ctx
            .archetypes
            .values()
            .find(|a| a.faction == faction);
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
            let mut bundle =
                generator.generate_with_rank(npc_uid, faction.clone(), *rank, pool, rng);
            // Roll a loadout for this member from the per-rank pool.
            bundle.loadout = generate_loadout(arch, bundle.xp.npc_rank(), loadout_ctx.items, rng);

            member_uids.push(npc_uid);
            if slot_idx == 0 || *rank > highest_rank {
                leader_uid = Some(npc_uid);
                highest_rank = *rank;
            }
            spawn.npcs.push(bundle);
        }

        let Some(leader) = leader_uid else { continue };

        // Resolve the template's coarse goal kind into a concrete Goal.
        // Squads prefer areas their own faction controls; only when no
        // owned area exists do they fall back to a random one. Keeps
        // most patrols on home turf so neighbouring factions' squads
        // only collide at territory boundaries.
        let goal = resolve_goal(template.goal, &faction, area_ids, loadout_ctx.areas, rng);

        // For Patrol/Scavenge goals, scatter 3 waypoints inside the
        // target area so multiple squads patrolling the same area don't
        // converge on a single point.
        let waypoints: Vec<[f32; 2]> = match &goal {
            Goal::Patrol { area } | Goal::Scavenge { area } => {
                roll_area_waypoints(area, loadout_ctx.areas, rng)
                    .into_iter()
                    .map(|v| [v.x, v.y])
                    .collect()
            }
            _ => Vec::new(),
        };

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
/// a target area, biased toward areas the squad's own faction
/// controls so squads cluster around home rather than wandering
/// random tiles of the map.
///
/// Falls back to any area if the faction holds no Settlements yet
/// (e.g. a faction present in data but not yet assigned territory).
fn resolve_goal<R: Rng>(
    kind: SquadGoalKind,
    faction: &Id<Faction>,
    area_ids: &[Id<Area>],
    areas: &HashMap<Id<Area>, AreaDef>,
    rng: &mut R,
) -> Goal {
    // Build the preferred pool: areas controlled by `faction` (only
    // Settlements carry a controller). If empty, use the full list.
    let owned: Vec<Id<Area>> = area_ids
        .iter()
        .filter(|id| {
            areas
                .get(*id)
                .and_then(|def| def.kind.faction())
                .is_some_and(|f| f == faction)
        })
        .cloned()
        .collect();
    let pool: &[Id<Area>] = if owned.is_empty() { area_ids } else { &owned };

    fn pick<R: Rng>(pool: &[Id<Area>], rng: &mut R) -> Option<Id<Area>> {
        if pool.is_empty() {
            None
        } else {
            Some(pool[rng.random_range(0..pool.len())].clone())
        }
    }
    match kind {
        SquadGoalKind::Idle => Goal::Idle,
        SquadGoalKind::Patrol => pick(pool, rng)
            .map(|area| Goal::Patrol { area })
            .unwrap_or(Goal::Idle),
        SquadGoalKind::Scavenge => pick(pool, rng)
            .map(|area| Goal::Scavenge { area })
            .unwrap_or(Goal::Idle),
    }
}

/// Weighted pick over `(faction_id, weight)` pairs. Each weight is
/// floored at 1 so a misconfigured `spawn_weight: 0` doesn't drop a
/// faction out of rotation entirely. `total_weight` is precomputed
/// by the caller so the loop doesn't re-sum on every roll.
fn pick_weighted_faction<R: Rng>(
    factions: &[(Id<Faction>, u32)],
    total_weight: u32,
    rng: &mut R,
) -> Id<Faction> {
    if total_weight == 0 {
        // Fallback shouldn't be reachable since the caller's empty
        // check runs first, but stay defensive.
        return factions[0].0.clone();
    }
    let mut roll = rng.random_range(0..total_weight);
    for (id, weight) in factions {
        let w = (*weight).max(1);
        if roll < w {
            return id.clone();
        }
        roll -= w;
    }
    factions[factions.len() - 1].0.clone()
}
