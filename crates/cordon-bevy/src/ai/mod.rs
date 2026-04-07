//! NPC behavior and AI.

pub mod behavior;
pub mod combat;
pub mod death;
pub mod loot;
pub mod squad;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

use self::behavior::Action;
use crate::AppState;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BehaviorPlugin::<Action>::default());
        app.add_plugins((
            combat::CombatPlugin,
            death::DeathPlugin,
            loot::LootPlugin,
            squad::SquadPlugin,
        ));
        app.add_systems(
            Update,
            (transition::<Action>, behavior::drive_actions)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
    }
}
