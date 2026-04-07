//! Death and corpse lifecycle.
//!
//! [`handle_deaths`] tags an NPC entity with [`Dead`] when its HP hits
//! zero, removes the round dot mesh, and spawns two crossed bars to
//! render an X marker. [`cleanup_corpses`] then despawns the entity
//! once its loadout is empty (fully looted) or after
//! [`CORPSE_PERSISTENCE_MINUTES`] of game time has elapsed.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;

use crate::PlayingState;
use crate::laptop::NpcDot;
use crate::world::SimWorld;

/// Marker for a corpse with its time of death.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
}

/// How long corpses persist before despawn (7 in-game days).
pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

/// Length of one bar of the dead-NPC X marker, in map units.
const X_BAR_LENGTH: f32 = 10.0;
/// Thickness of one bar of the X marker, in map units.
const X_BAR_THICKNESS: f32 = 1.5;
/// Mid-grey colour for the X marker.
const X_BAR_COLOR: Color = Color::srgba(0.55, 0.55, 0.55, 0.9);

/// Plugin registering the death/cleanup systems.
pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_deaths, cleanup_corpses)
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Tag NPCs whose health hit zero as dead and replace their dot with
/// a standalone X mark (no background circle).
fn handle_deaths(
    sim: Option<Res<SimWorld>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q: Query<(Entity, &NpcDot), Without<Dead>>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, npc_dot) in &q {
        let Some(npc) = sim.0.npcs.get(&npc_dot.uid) else {
            continue;
        };
        if npc.health.is_alive() {
            continue;
        }
        commands
            .entity(entity)
            .insert(Dead { died_at: now })
            // Remove the round dot mesh entirely; the X bars below stand alone.
            .remove::<Mesh2d>()
            .remove::<MeshMaterial2d<ColorMaterial>>();

        // Two crossed bar children form the X marker.
        let bar_mesh = meshes.add(Rectangle::new(X_BAR_LENGTH, X_BAR_THICKNESS));
        let bar_mat = materials.add(ColorMaterial::from_color(X_BAR_COLOR));
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh2d(bar_mesh.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ));
            parent.spawn((
                Mesh2d(bar_mesh.clone()),
                MeshMaterial2d(bar_mat.clone()),
                Transform::from_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4)),
            ));
        });
    }
}

/// Despawn corpses after the persistence window or once their loadout
/// has been fully looted.
fn cleanup_corpses(
    sim: Option<Res<SimWorld>>,
    mut commands: Commands,
    q: Query<(Entity, &Dead, &NpcDot)>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, dead, npc_dot) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = sim
            .0
            .npcs
            .get(&npc_dot.uid)
            .map(|n| n.loadout.is_empty())
            .unwrap_or(true);
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            commands.entity(entity).despawn();
        }
    }
}

/// Convert a [`GameTime`] to absolute minutes since day 1, 00:00.
fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

/// Game-minutes elapsed between two times. Saturating; never negative.
fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
}
