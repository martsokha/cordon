//! Achievement tracking systems. Each system watches a game event
//! and writes [`UnlockAchievement`] when the condition is met.

use bevy::prelude::*;
use bevy_steamworks::Client;

use super::UnlockAchievement;
use super::achievements::Achievement;

/// Forward [`UnlockAchievement`] messages to Steam.
pub fn process_achievements(client: Res<Client>, mut unlocks: MessageReader<UnlockAchievement>) {
    let user_stats = client.user_stats();
    let mut any = false;

    for unlock in unlocks.read() {
        let name = unlock.0.api_name();
        let ach = user_stats.achievement(name);
        if ach.get().unwrap_or(false) {
            continue;
        }
        if ach.set().is_ok() {
            info!("Steam achievement unlocked: {name}");
            any = true;
        } else {
            warn!("Failed to set Steam achievement: {name}");
        }
    }

    if any {
        let _ = user_stats.store_stats();
    }
}

pub fn track_first_kill(
    mut died: MessageReader<cordon_sim::death::NpcDied>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    if died.read().next().is_some() {
        unlocks.write(UnlockAchievement(Achievement::FirstKill));
    }
}

pub fn track_squad_wipe(
    mut died: MessageReader<cordon_sim::death::NpcDied>,
    squads: Query<&cordon_sim::plugin::prelude::SquadMembers>,
    dead: Query<(), With<cordon_sim::behavior::Dead>>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    for event in died.read() {
        for members in squads.iter() {
            if !members.0.contains(&event.entity) {
                continue;
            }
            if members.0.iter().all(|e| dead.contains(*e)) {
                unlocks.write(UnlockAchievement(Achievement::SquadWipe));
            }
        }
    }
}

pub fn track_first_relic(
    mut picked: MessageReader<cordon_sim::spawn::relics::RelicPickedUp>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    if picked.read().next().is_some() {
        unlocks.write(UnlockAchievement(Achievement::FirstRelic));
    }
}

pub fn track_cctv_peek(
    mode: Res<crate::bunker::CameraMode>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    if matches!(*mode, crate::bunker::CameraMode::AtCctv { .. }) {
        unlocks.write(UnlockAchievement(Achievement::CctvPeek));
    }
}

pub fn track_open_for_business(
    state: Res<crate::bunker::VisitorState>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    if matches!(*state, crate::bunker::VisitorState::Inside { .. }) {
        unlocks.write(UnlockAchievement(Achievement::OpenForBusiness));
    }
}

pub fn track_survive_7(
    clock: Option<Res<cordon_sim::resources::GameClock>>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    let Some(clock) = clock else { return };
    if clock.0.day.value() >= 7 {
        unlocks.write(UnlockAchievement(Achievement::Survive7));
    }
}

pub fn track_first_quest(
    quest_log: Option<Res<cordon_sim::quest::QuestLog>>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    let Some(log) = quest_log else { return };
    if !log.completed.is_empty() {
        unlocks.write(UnlockAchievement(Achievement::FirstQuest));
    }
}

pub fn track_rich(
    player: Option<Res<cordon_sim::resources::Player>>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    let Some(player) = player else { return };
    if player.0.credits.value() >= 10_000 {
        unlocks.write(UnlockAchievement(Achievement::Rich));
    }
}

pub fn track_explore_all(
    revealed: Option<Res<crate::laptop::fog::RevealedAreas>>,
    areas: Option<Res<cordon_sim::resources::AreaStates>>,
    mut unlocks: MessageWriter<UnlockAchievement>,
) {
    let (Some(revealed), Some(areas)) = (revealed, areas) else {
        return;
    };
    if !areas.0.is_empty() && revealed.0.len() >= areas.0.len() {
        unlocks.write(UnlockAchievement(Achievement::ExploreAll));
    }
}
