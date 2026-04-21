//! Sleep system: the player interacts with the sofa in quarters
//! to sleep for 8 hours. The sim runs at 500× during the fade-
//! to-black, so the world continues (squads move, combat
//! resolves, events fire). Screen fades out, time passes, screen
//! fades back in.

mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct SleepPlugin;

impl Plugin for SleepPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<systems::SleepState>();
        app.add_systems(
            Update,
            (
                systems::attach_sleep_target,
                systems::attach_observer,
                systems::drive_sleep_transition,
                systems::gate_sleep_while_visitor_waiting,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
