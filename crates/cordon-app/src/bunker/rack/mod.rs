//! Rack storage: physical shelf slots that hold items.
//!
//! Each `StorageRack01` prop gets a [`Rack`] component with 3
//! per-shelf child entities ([`RackSlot`]). The player interacts
//! with individual slots to take, place, or swap items using the
//! standard [`Interactable`] system. One item is held at a time
//! via the [`Carrying`] resource, rendered as a box attached to
//! the FPS camera.

pub mod components;
mod shelf_prop;
mod systems;

use bevy::prelude::*;
pub use components::Carrying;
pub use systems::reset_rack_state;

use crate::PlayingState;

pub struct RackPlugin;

impl Plugin for RackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Carrying>();
        app.add_systems(Startup, systems::load_rack_sfx);
        app.add_systems(
            Update,
            (
                systems::spawn_rack_slots,
                systems::attach_slot_observers,
                systems::update_slot_prompts,
                systems::block_non_rack_interactions,
                systems::animate_carried_bob,
                systems::drop_carried,
                systems::populate_starter_items,
                systems::drain_pending_to_racks,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}
