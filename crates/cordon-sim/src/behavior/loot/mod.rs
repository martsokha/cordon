//! Looting: alive NPCs near a corpse pull items from its loadout into
//! their own general pouch.

pub mod components;
pub mod constants;
pub mod events;
pub mod systems;

use bevy::prelude::*;
pub use components::LootState;
pub use events::ItemLooted;

use crate::plugin::SimSet;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ItemLooted>();
        app.add_systems(
            Update,
            (systems::try_start_looting, systems::drive_loot)
                .chain()
                .in_set(SimSet::Loot),
        );
    }
}
