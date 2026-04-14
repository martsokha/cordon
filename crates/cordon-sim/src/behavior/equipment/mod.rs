//! Equipment side-effects subplugin.
//!
//! Loadout changes (equip / unequip / drop a relic) are data
//! mutations in cordon-core, but they need to ripple into ECS
//! state — today that's just pool-cap resync; in future this is
//! where timed passives, equip-triggered events, and drop-triggered
//! cleanup all belong. Runs in [`SimSet::Cleanup`] so the rest of
//! the frame sees the recomputed values.

pub mod systems;

use bevy::prelude::*;

use crate::plugin::SimSet;

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        // Runs in Cleanup: early enough that the rest of the frame
        // sees the updated max, late enough that the pickup from
        // the previous frame has already landed in the loadout.
        app.add_systems(Update, systems::sync_pool_maxes.in_set(SimSet::Cleanup));
    }
}
