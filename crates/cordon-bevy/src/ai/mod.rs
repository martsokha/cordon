//! NPC behavior and AI.

pub mod behavior;

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

use crate::AppState;
use behavior::VisitorBehavior;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BehaviorPlugin::<VisitorBehavior>::default());
        app.add_systems(
            Update,
            (
                transition::<VisitorBehavior>,
                behavior::drive_visitor_behavior,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
    }
}
