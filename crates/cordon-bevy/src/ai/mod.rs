//! NPC behavior and AI.

pub mod behavior;
pub mod combat;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

use self::behavior::Action;
use crate::AppState;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BehaviorPlugin::<Action>::default());
        app.add_plugins(combat::CombatPlugin);
        app.add_systems(
            Update,
            (
                transition::<Action>,
                behavior::drive_actions,
                behavior::drive_intents,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
    }
}
