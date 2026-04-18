//! Top-level Bevy resources owned by `cordon-sim`, plus the
//! world-bootstrap function that populates them.
//!
//! Each concern is its own resource so systems declare exactly
//! what they touch and Bevy can run them in parallel where
//! possible. [`init_world_resources`] is called once by the
//! cordon-app layer on `OnEnter(AppState::Playing)` and fills
//! every resource defined below from loaded [`GameDataResource`].

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::bunker::{Upgrade, UpgradeDef, UpgradeEffect};
use cordon_core::entity::faction::Faction;
use cordon_core::entity::player::{PlayerRank, PlayerState};
use cordon_core::entity::squad::Squad;
use cordon_core::item::{Item, ItemInstance, Stash, StashScope};
use cordon_core::primitive::{Credits, Day, Experience, GameTime, Id, Relation, Uid};
use cordon_core::world::area::{Area, AreaKind};
use cordon_core::world::narrative::{ActiveEvent, Intel, IntelDef};
use cordon_data::gamedata::GameDataResource;

/// Live state of an area in the world.
///
/// Tracks dynamic properties that change during gameplay: faction
/// control, danger modifiers from events, creature activity. Base
/// danger/reward values come from the area's config definition.
pub struct AreaState {
    pub id: Id<Area>,
    /// Which faction currently controls this area, if any.
    pub controlling_faction: Option<Id<Faction>>,
    /// Additive danger modifier from events/world state.
    pub danger_modifier: f32,
    /// Creature activity level (0.0–1.0). Affects danger.
    pub creature_activity: f32,
    /// Whether a hazard field is currently active.
    pub hazard_active: bool,
}

impl AreaState {
    pub fn new(id: Id<Area>) -> Self {
        Self {
            id,
            controlling_faction: None,
            danger_modifier: 0.0,
            creature_activity: 0.0,
            hazard_active: false,
        }
    }
}

/// Maps stable squad uids to their current ECS entity. Maintained by
/// the spawn system and used by AI systems for the rare uid → entity
/// lookups (e.g. resolving `Goal::Protect { other }`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SquadIdIndex(pub HashMap<Uid<Squad>, Entity>);

/// Per-hire bookkeeping for one squad on the player's roster.
///
/// Empty for now — the field exists so future per-hire metadata
/// (date hired, custom callsign, assignment) can be added without
/// changing [`PlayerSquadRoster`]'s shape.
///
/// Daily pay is intentionally **not** stored here — it's a pure
/// function of the squad's current member ranks (see
/// [`Rank::pay`](cordon_core::primitive::Rank::pay)) so member
/// deaths immediately reduce the bill with no recompute system.
#[derive(Debug, Clone, Default)]
pub struct PlayerSquadEntry {}

/// All squads the player has hired. Squads are the only unit of
/// player ownership — there is no individual NPC hiring.
///
/// Keyed by stable [`Uid<Squad>`] so the roster survives respawns
/// and is save-ready. ECS systems that need entity access find
/// the live entity via [`SquadIdIndex`] (or, more commonly, through
/// the derived [`Owned`](crate::behavior::squad::Owned) marker
/// which is kept in sync by [`sync_owned_marker`]).
#[derive(Resource, Default, Debug, Clone)]
pub struct PlayerSquadRoster {
    entries: HashMap<Uid<Squad>, PlayerSquadEntry>,
}

impl PlayerSquadRoster {
    /// Add a squad to the roster. No-op if already hired.
    pub fn hire(&mut self, squad: Uid<Squad>) {
        self.entries.entry(squad).or_default();
    }

    /// Remove a squad from the roster. No-op if not hired.
    pub fn dismiss(&mut self, squad: &Uid<Squad>) {
        self.entries.remove(squad);
    }

    pub fn is_hired(&self, squad: &Uid<Squad>) -> bool {
        self.entries.contains_key(squad)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Uid<Squad>, &PlayerSquadEntry)> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// In-game clock. Advanced by `cordon_app::world::tick_game_time`.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct GameClock(pub GameTime);

/// XP, credits, debt — the player's numeric identity.
#[derive(Resource, Debug, Clone)]
pub struct PlayerIdentity {
    pub xp: Experience,
    pub credits: Credits,
    pub debt: Credits,
}

impl PlayerIdentity {
    /// Current rank, derived from XP.
    pub fn rank(&self) -> PlayerRank {
        PlayerRank::from_xp(self.xp)
    }

    /// Add experience points.
    pub fn add_xp(&mut self, amount: u32) {
        self.xp.add(amount);
    }

    /// Whether the player can afford a given cost.
    pub fn can_afford(&self, amount: Credits) -> bool {
        self.credits.can_afford(amount)
    }
}

/// Faction relations.
#[derive(Resource, Debug, Clone)]
pub struct PlayerStandings {
    pub standings: Vec<(Id<Faction>, Relation)>,
}

impl PlayerStandings {
    /// Get the player's standing with a faction.
    pub fn standing(&self, faction: &Id<Faction>) -> Relation {
        self.standings
            .iter()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Get a mutable reference to the player's standing with a faction.
    pub fn standing_mut(&mut self, faction: &Id<Faction>) -> Option<&mut Relation> {
        self.standings
            .iter_mut()
            .find(|(f, _)| f == faction)
            .map(|(_, s)| s)
    }
}

/// Installed bunker/camp upgrades.
#[derive(Resource, Debug, Clone)]
pub struct PlayerUpgrades {
    pub upgrades: Vec<Id<Upgrade>>,
}

impl PlayerUpgrades {
    /// Check if an upgrade is installed (bunker or camp).
    pub fn has_upgrade(&self, upgrade_id: &Id<Upgrade>) -> bool {
        self.upgrades.iter().any(|u| u == upgrade_id)
    }

    /// Iterate every [`UpgradeEffect`] granted by the player's
    /// currently-installed upgrades, resolved against the game
    /// data catalog.
    pub fn installed_effects<'a>(
        &'a self,
        upgrades: &'a HashMap<Id<Upgrade>, UpgradeDef>,
    ) -> impl Iterator<Item = &'a UpgradeEffect> + 'a {
        self.upgrades
            .iter()
            .filter_map(|id| upgrades.get(id))
            .flat_map(|def| def.effects.iter())
    }
}

/// Item staging queue + hidden storage.
#[derive(Resource, Debug, Clone)]
pub struct PlayerStash {
    pub pending_items: Stash,
    pub hidden_storage: Stash,
}

impl PlayerStash {
    /// Insert an item instance into the requested scope.
    pub fn add_item(&mut self, instance: ItemInstance, scope: StashScope) {
        match scope {
            StashScope::Main | StashScope::Any => self.pending_items.add(instance),
            StashScope::Hidden => self.hidden_storage.add(instance),
        }
    }

    /// Remove and return the first instance of the given item def
    /// within the scope, or `None` if nothing matches.
    pub fn remove_first(&mut self, item: &Id<Item>, scope: StashScope) -> Option<ItemInstance> {
        let take_from = |stash: &mut Stash| -> Option<ItemInstance> {
            let index = stash.items().iter().position(|i| &i.def_id == item)?;
            stash.remove(index)
        };
        match scope {
            StashScope::Main => take_from(&mut self.pending_items),
            StashScope::Hidden => take_from(&mut self.hidden_storage),
            StashScope::Any => {
                take_from(&mut self.pending_items).or_else(|| take_from(&mut self.hidden_storage))
            }
        }
    }

    /// Total count of a given item definition across the requested scope.
    pub fn item_count(&self, item: &Id<Item>, scope: StashScope) -> u32 {
        let sum = |stash: &Stash| -> u32 {
            stash
                .items()
                .iter()
                .filter(|i| &i.def_id == item)
                .map(|i| i.count)
                .sum()
        };
        match scope {
            StashScope::Main => sum(&self.pending_items),
            StashScope::Hidden => sum(&self.hidden_storage),
            StashScope::Any => sum(&self.pending_items) + sum(&self.hidden_storage),
        }
    }

    /// Whether the player holds at least `count` of the given item
    /// def within the scope.
    pub fn has_item(&self, item: &Id<Item>, count: u32, scope: StashScope) -> bool {
        self.item_count(item, scope) >= count
    }
}

/// Assemble a full [`PlayerState`] DTO from the four sub-resources.
/// Used for save/load serialisation.
pub fn assemble_player_state(
    id: &PlayerIdentity,
    st: &PlayerStandings,
    up: &PlayerUpgrades,
    stash: &PlayerStash,
) -> PlayerState {
    PlayerState {
        xp: id.xp,
        credits: id.credits,
        debt: id.debt,
        standings: st.standings.clone(),
        upgrades: up.upgrades.clone(),
        pending_items: stash.pending_items.clone(),
        hidden_storage: stash.hidden_storage.clone(),
    }
}

/// All faction IDs from config paired with their spawn weight, used
/// for weighted faction selection during NPC generation. Built once
/// at world init from `GameDataResource`. The [`FactionDef`] field
/// `spawn_weight` controls how often each faction is rolled.
#[derive(Resource, Debug, Clone, Default)]
pub struct FactionIndex(pub Vec<(Id<Faction>, u32)>);

/// Pre-collected centres of every Settlement-archetype area, indexed
/// by controlling faction. Built once at world init so the spawn
/// system doesn't have to walk every area every wave to figure out
/// where a faction's bases are.
#[derive(Resource, Debug, Clone, Default)]
pub struct FactionSettlements(pub HashMap<Id<Faction>, Vec<bevy::math::Vec2>>);

/// Live area states keyed by area id. Tracks faction control, danger,
/// creature activity.
#[derive(Resource, Default)]
pub struct AreaStates(pub HashMap<Id<Area>, AreaState>);

/// All currently-active environmental/economic/faction/personal
/// events. Rolled daily; expired entries pruned at the day rollover.
#[derive(Resource, Debug, Clone, Default)]
pub struct EventLog(pub Vec<ActiveEvent>);

/// One piece of intel the player has discovered.
#[derive(Debug, Clone)]
pub struct KnownIntel {
    /// Which intel definition this is.
    pub id: Id<Intel>,
    /// The day the player learned this intel.
    pub day_acquired: Day,
}

/// All intel entries the player currently knows. Populated by
/// radio broadcasts, quest consequences, and dialogue.
#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerIntel {
    pub entries: Vec<KnownIntel>,
}

impl PlayerIntel {
    /// Whether the player already knows this intel entry.
    pub fn has(&self, id: &Id<Intel>) -> bool {
        self.entries.iter().any(|e| &e.id == id)
    }

    /// Grant an intel entry. No-op if already known.
    pub fn grant(&mut self, id: Id<Intel>, day: Day) {
        if !self.has(&id) {
            self.entries.push(KnownIntel {
                id,
                day_acquired: day,
            });
        }
    }

    /// Remove expired entries given the current day and the intel
    /// catalog. Entries whose definition has `expires_after: Some(d)`
    /// are pruned when the elapsed days since acquisition exceed `d`
    /// converted to whole days. Runs on day rollover, so day
    /// granularity is appropriate.
    pub fn expire(&mut self, current_day: Day, defs: &HashMap<Id<Intel>, IntelDef>) {
        self.entries.retain(|entry| {
            let Some(def) = defs.get(&entry.id) else {
                return true;
            };
            let Some(ttl) = def.expires_after else {
                return true;
            };
            let elapsed_days = current_day
                .value()
                .saturating_sub(entry.day_acquired.value());
            // Duration stores minutes; convert to whole days
            // (rounding up so a 1-hour TTL still survives at
            // least until the next day rollover).
            let ttl_days = (ttl.minutes() + 24 * 60 - 1) / (24 * 60);
            elapsed_days < ttl_days
        });
    }
}

/// Tracks when the player last took pills. `None` means never,
/// which the quest system interprets as "no doses since game
/// start" so the "n days without pills" trigger can fire on a
/// fresh run.
///
/// Stored as a full [`GameTime`] rather than just [`Day`] so the
/// "days without pills" check measures real elapsed time (game
/// minutes / 1440), not a day-number diff. That way taking pills
/// at 23:59 on day 1 doesn't count as a full day without pills
/// at 00:01 on day 2.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PlayerPills {
    pub last_taken: Option<GameTime>,
}

impl PlayerPills {
    /// Whole 24-hour spans elapsed since the last dose, or since
    /// the [`GameTime::new`] origin when the player has never
    /// taken pills.
    pub fn days_without(&self, now: GameTime) -> u32 {
        let baseline = self.last_taken.unwrap_or_else(GameTime::new);
        now.minutes_since(baseline) / (24 * 60)
    }

    /// Stamp a dose at the given moment.
    pub fn record_dose(&mut self, now: GameTime) {
        self.last_taken = Some(now);
    }
}

/// Monotonic Uid allocator. Each call to [`UidAllocator::alloc`]
/// returns a fresh `Uid<T>` typed for the caller's marker.
#[derive(Resource, Debug, Clone)]
pub struct UidAllocator {
    next: u32,
}

impl Default for UidAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl UidAllocator {
    pub fn alloc<T: Send + Sync + 'static>(&mut self) -> Uid<T> {
        let uid = Uid::new(self.next);
        self.next += 1;
        uid
    }
}

/// Per-frame fractional accumulator for [`tick_game_time`]. Keeps
/// sub-minute progress between frames so the clock doesn't
/// discretely jump whenever a whole minute happens to align with
/// a frame boundary.
#[derive(Resource, Default, Debug)]
pub struct TimeAccumulator(pub f32);

/// How many game minutes pass per real second at 1× sim speed.
/// A game day at this rate is 12 real minutes.
const GAME_MINUTES_PER_SECOND: f32 = 2.0;

/// Marker type for simulation time. Advanced each frame by
/// [`tick_sim_time`] from virtual time scaled by [`SimSpeed`].
/// Decoupled from `Time<Virtual>` so we can accelerate the sim
/// (e.g. during sleep) without causing the `FixedMain` loop to
/// explode into dozens of physics/transform ticks per frame.
///
/// Every sim system reads `Res<Time<Sim>>` instead of `Res<Time>`
/// so it sees the scaled delta.
#[derive(Default, Debug, Clone, Copy)]
pub struct Sim;

/// Simulation speed multiplier. 1.0 = normal play. Set to e.g.
/// 50.0 during sleep to fast-forward the sim while the screen is
/// black. `Time<Virtual>` stays at 1×; only `Time<Sim>` sees the
/// speedup.
#[derive(Resource, Debug, Clone, Copy)]
pub struct SimSpeed(pub f64);

impl Default for SimSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Advance `Time<Sim>` each frame from virtual time × sim speed.
pub fn tick_sim_time(
    virtual_time: Res<Time<Virtual>>,
    mut sim_time: ResMut<Time<Sim>>,
    speed: Res<SimSpeed>,
) {
    let delta = virtual_time.delta().mul_f64(speed.0);
    sim_time.advance_by(delta);
}

/// Build the cordon-sim resource set from loaded game data. Also
/// valid as a "reset to a fresh run" — every `insert_resource`
/// below replaces rather than merges, so calling this mid-session
/// wipes any in-flight state (player stash, faction standings,
/// quest log, intel, etc.). The caller handles entity despawns
/// (NPCs, squads, relics) and the one-time catalog side of the
/// setup elsewhere.
pub fn init_world_resources(mut commands: Commands, game_data: Res<GameDataResource>) {
    let data = &game_data.0;

    let faction_ids = data.faction_ids();
    // Pair each faction with its spawn weight from config so the
    // spawn system can do a weighted pick without re-reading the
    // FactionDef catalog every wave.
    let faction_weights: Vec<(_, u32)> = faction_ids
        .iter()
        .map(|id| {
            let weight = data.factions.get(id).map(|f| f.spawn_weight).unwrap_or(1);
            (id.clone(), weight)
        })
        .collect();

    let mut areas: HashMap<_, _> = HashMap::with_capacity(data.areas.len());
    for id in data.area_ids() {
        areas.insert(id.clone(), AreaState::new(id.clone()));
    }

    // Pre-collect each faction's Settlement centres so the spawn
    // system doesn't have to walk every area every wave. Built once
    // here because settlements are static config — they don't
    // change at runtime.
    let mut settlements: HashMap<_, Vec<Vec2>> = HashMap::with_capacity(faction_ids.len());
    for area in data.areas.values() {
        if let AreaKind::Settlement { faction, .. } = &area.kind {
            settlements
                .entry(faction.clone())
                .or_default()
                .push(Vec2::new(area.location.x, area.location.y));
        }
    }

    let player_state = PlayerState::new(&faction_ids);

    commands.insert_resource(GameClock::default());
    commands.insert_resource(PlayerIdentity {
        xp: player_state.xp,
        credits: player_state.credits,
        debt: player_state.debt,
    });
    commands.insert_resource(PlayerStandings {
        standings: player_state.standings,
    });
    commands.insert_resource(PlayerUpgrades {
        upgrades: player_state.upgrades,
    });
    commands.insert_resource(PlayerStash {
        pending_items: player_state.pending_items,
        hidden_storage: player_state.hidden_storage,
    });
    commands.insert_resource(PlayerSquadRoster::default());
    commands.insert_resource(SquadIdIndex::default());
    commands.insert_resource(FactionIndex(faction_weights));
    commands.insert_resource(FactionSettlements(settlements));
    commands.insert_resource(AreaStates(areas));
    commands.insert_resource(EventLog::default());
    commands.insert_resource(PlayerIntel::default());
    commands.insert_resource(PlayerPills::default());
    commands.insert_resource(TimeAccumulator::default());
    // Quest + day-cycle resources initialised by their own plugins on
    // app build, but we re-insert them here so mid-session reset
    // wipes their accumulated state (active quests, dead-template
    // flags, prior-day broadcast dedup, prior-day debt, timer hands
    // anchored to entities we just despawned).
    commands.insert_resource(crate::quest::QuestLog::default());
    commands.insert_resource(crate::quest::TemplateRegistry::default());
    commands.insert_resource(crate::day::payroll::LastDailyExpenses::default());
    commands.insert_resource(crate::day::radio::DeliveredBroadcasts::default());
    commands.insert_resource(crate::behavior::effects::PeriodicTriggers::default());

    info!("World initialised; population will be spawned by cordon-sim");
}

/// Per-frame clock advance. Reads `Time<Sim>` so the game clock
/// scales with [`SimSpeed`] instead of `Time<Virtual>`. This
/// means sleep acceleration doesn't touch virtual time and
/// doesn't disturb the `FixedMain` loop.
pub fn tick_game_time(
    time: Res<Time<Sim>>,
    mut acc: ResMut<TimeAccumulator>,
    mut clock: ResMut<GameClock>,
) {
    acc.0 += time.delta_secs() * GAME_MINUTES_PER_SECOND;
    let minutes = acc.0 as u32;
    if minutes > 0 {
        acc.0 -= minutes as f32;
        clock.0.advance_minutes(minutes);
    }
}
