//! NPC behavior and AI.
//!
//! All AI systems share a single ordered [`AiSet`] schedule so cross-
//! plugin systems run in a well-defined order each frame:
//!
//! 1. **Cleanup** — prune despawned squad members, expire tracers
//! 2. **Goals** — Hold timer expires → next goal-driven activity
//! 3. **Engagement** — squad vision scan, set per-NPC `CombatTarget`
//! 4. **Formation** — write per-NPC `MovementTarget` from formation slots
//! 5. **Movement** — apply `MovementTarget` to `Transform`
//! 6. **Combat** — read `CombatTarget`, fire shots, apply damage
//! 7. **Death** — mark NPCs whose HP hit zero
//! 8. **Loot** — adjacent looters pull items from corpses

pub mod behavior;
pub mod combat;
pub mod death;
pub mod loot;
pub mod squad;

use bevy::prelude::*;

use crate::AppState;

/// Ordered system set for the entire AI pipeline. Systems declare
/// `.in_set(AiSet::X)` and the chain enforces the schedule above.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiSet {
    Cleanup,
    Goals,
    Engagement,
    Formation,
    Movement,
    Combat,
    Death,
    Loot,
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // Configure the order of the AI sets. `.chain()` enforces
        // strict sequential execution between sets, while still
        // allowing parallelism *within* a set.
        app.configure_sets(
            Update,
            (
                AiSet::Cleanup,
                AiSet::Goals,
                AiSet::Engagement,
                AiSet::Formation,
                AiSet::Movement,
                AiSet::Combat,
                AiSet::Death,
                AiSet::Loot,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        );

        app.add_plugins((
            combat::CombatPlugin,
            death::DeathPlugin,
            loot::LootPlugin,
            squad::SquadPlugin,
        ));

        // The single per-NPC movement system lives in this module so
        // we don't pull a tiny plugin in for one function.
        app.add_systems(
            Update,
            behavior::move_npcs
                .in_set(AiSet::Movement)
                .run_if(in_state(AppState::Playing)),
        );
    }
}
