//! Steam integration: SDK init + achievement tracking.
//!
//! Achievement triggers are scattered across the sim and bevy
//! layers; each writes an [`UnlockAchievement`] message which
//! [`systems::process_achievements`] picks up and forwards to
//! Steam.

pub mod achievements;
mod systems;

use bevy::prelude::*;
use bevy_steamworks::*;

use self::achievements::Achievement;

/// Message written by game systems when an achievement should be
/// unlocked. The Steam system picks these up each frame.
#[derive(Message, Debug, Clone, Copy)]
pub struct UnlockAchievement(pub Achievement);

pub struct SteamPlugin;

impl Plugin for SteamPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SteamworksPlugin::init_app(480).unwrap());
        app.add_message::<UnlockAchievement>();
        app.add_systems(
            Update,
            (
                systems::process_achievements,
                systems::track_first_kill,
                systems::track_squad_wipe,
                systems::track_first_relic,
                systems::track_cctv_peek,
                systems::track_open_for_business,
                systems::track_survive_7,
                systems::track_first_quest,
                systems::track_rich,
                systems::track_explore_all,
            ),
        );
    }
}
