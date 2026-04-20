//! Top-level Bevy resources owned by `cordon-sim`, split by
//! concern and re-exported flat so callers keep saying
//! `use cordon_sim::resources::Foo`.
//!
//! - [`player`] — identity, standings, upgrades, stash, intel,
//!   squad roster, save-state assembly.
//! - [`world`] — per-area runtime state, faction weighting for
//!   spawning, settlement positions, active events.
//! - [`clock`] — game clock, `Time<Sim>`, sim-speed multiplier,
//!   Uid allocator, squad-uid → entity index.
//!
//! [`init_world_resources`] composes every reset-on-run resource
//! in one place so `OnEnter(AppState::Playing)` can wipe a run
//! back to a fresh start.

pub mod clock;
pub mod player;
pub mod world;

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::player::PlayerState;
use cordon_core::world::area::AreaKind;
use cordon_data::gamedata::GameDataResource;

pub use self::clock::{
    GameClock, Sim, SimSpeed, SquadIdIndex, TimeAccumulator, UidAllocator, tick_game_time,
    tick_sim_time,
};
pub use self::player::{
    KnownIntel, PlayerDecisions, PlayerIdentity, PlayerIntel, PlayerSquadEntry, PlayerSquadRoster,
    PlayerStandings, PlayerStash, PlayerUpgrades, assemble_player_state,
};
pub use self::world::{AreaState, AreaStates, EventLog, FactionIndex, FactionSettlements};

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
    commands.insert_resource(PlayerDecisions::default());
    commands.insert_resource(crate::bunker::pills::PlayerPills::default());
    commands.insert_resource(TimeAccumulator::default());
    // Quest + day-cycle resources initialised by their own plugins on
    // app build, but we re-insert them here so mid-session reset
    // wipes their accumulated state (active quests, dead-template
    // flags, prior-day broadcast dedup, prior-day debt, timer hands
    // anchored to entities we just despawned).
    commands.insert_resource(crate::quest::QuestLog::default());
    commands.insert_resource(crate::quest::TemplateRegistry::default());
    commands.insert_resource(crate::quest::dispatch::QuestDispatchState::default());
    commands.insert_resource(crate::day::payroll::LastDailyExpenses::default());
    commands.insert_resource(crate::day::radio::DeliveredBroadcasts::default());
    commands.insert_resource(crate::behavior::effects::PeriodicTriggers::default());

    info!("World initialised; population will be spawned by cordon-sim");
}
