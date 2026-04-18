//! Medication interaction: the player can take pills from any
//! `MedicationCluster1` prop in the bunker. No gameplay effect
//! yet — just a sound and a log line. The cluster stays on the
//! table (infinite supply) so the interaction is always available.

mod systems;

use bevy::prelude::*;

use crate::PlayingState;

pub struct PillsPlugin;

impl Plugin for PillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::load_sfx);
        app.add_systems(
            Update,
            (systems::attach_interactable, systems::attach_observer)
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
