//! Entity-behavior subplugins.
//!
//! Grouped here so the top-level crate has one place for "everything
//! that controls how a spawned entity acts": movement, vision,
//! combat, death, loot, effects, equipment, squad. Each subplugin
//! follows the `{mod.rs, components.rs, systems.rs, events.rs,
//! constants.rs}` shape (with files omitted when empty), so you can
//! find any component / system / event by folder + filename.
//!
//! [`squad`] has enough internal structure (engagement, formation,
//! lifecycle, commands, behave trees) that it uses a feature-file
//! layout — each file named for the concern it owns — alongside
//! shared `constants.rs` and dedicated `identity`, `intent`, and
//! `formation` files for the components each concern writes.
//!
//! # System ordering
//!
//! Each subplugin registers its own systems with `.in_set(SimSet::X)`.
//! The inter-subplugin order is enforced by the `.chain()` in
//! [`crate::plugin`] between `SimSet::Commands → Cleanup → Spawn →
//! Goals → Engagement → Formation → Movement → Combat → Effects →
//! Death → Loot`. Adding a new subplugin that needs to slot into the
//! frame must either pick an existing `SimSet` or extend the enum
//! and update the `.chain()` site — the subplugin itself cannot
//! reorder the sets.

pub mod combat;
pub mod death;
pub mod effects;
pub mod equipment;
pub mod loot;
pub mod movement;
pub mod squad;
pub mod vision;

use bevy::prelude::*;

/// Composer plugin that wires up every behavior subplugin.
pub struct BehaviorPlugin;

impl Plugin for BehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            vision::VisionPlugin,
            movement::MovementPlugin,
            combat::CombatPlugin,
            death::DeathPlugin,
            loot::LootPlugin,
            effects::EffectsPlugin,
            equipment::EquipmentPlugin,
            squad::SquadPlugin,
        ));
    }
}
