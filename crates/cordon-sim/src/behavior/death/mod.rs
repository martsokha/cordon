//! Death and corpse lifecycle subplugin.

pub mod components;
pub mod constants;
pub mod events;
pub mod systems;

use bevy::prelude::*;

pub use components::Dead;
pub use events::{CorpseRemoved, NpcDied};

use crate::plugin::SimSet;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NpcDied>();
        app.add_message::<CorpseRemoved>();
        // `handle_deaths` runs every tick so newly-dead NPCs tag in
        // the same frame combat detected the kill — visual layers and
        // event consumers expect that immediacy. The two cleanup
        // systems are pure housekeeping, so they throttle down to
        // once per second to keep the scan off the hot path.
        app.add_systems(
            Update,
            (
                systems::handle_deaths,
                (systems::cleanup_corpses, systems::enforce_corpse_cap)
                    .chain()
                    .run_if(systems::on_cleanup_tick),
            )
                .chain()
                .in_set(SimSet::Death),
        );
    }
}
