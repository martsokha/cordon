//! Death and corpse lifecycle.
//!
//! [`handle_deaths`] tags an NPC entity with [`Dead`] when its HP hits
//! zero, removes the round dot mesh, and spawns two crossed bars to
//! render an X marker. [`cleanup_corpses`] then despawns the entity
//! once its loadout is empty (fully looted) or after
//! [`CORPSE_PERSISTENCE_MINUTES`] of game time has elapsed.

use bevy::prelude::*;
use cordon_core::primitive::GameTime;
use cordon_sim::components::{Hp, LoadoutComp, NpcMarker};

use super::AiSet;
use crate::PlayingState;
use crate::world::SimWorld;

/// Marker for a corpse with its time of death.
#[derive(Component, Debug, Clone, Copy)]
pub struct Dead {
    pub died_at: GameTime,
}

pub const CORPSE_PERSISTENCE_MINUTES: u32 = 7 * 24 * 60;

const X_BAR_LENGTH: f32 = 10.0;
const X_BAR_THICKNESS: f32 = 1.5;
const X_BAR_COLOR: Color = Color::srgba(0.55, 0.55, 0.55, 0.9);

#[derive(Resource, Clone)]
pub struct DeathAssets {
    pub bar_mesh: Handle<Mesh>,
    pub bar_mat: Handle<ColorMaterial>,
}

fn init_death_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let bar_mesh = meshes.add(Rectangle::new(X_BAR_LENGTH, X_BAR_THICKNESS));
    let bar_mat = materials.add(ColorMaterial::from_color(X_BAR_COLOR));
    commands.insert_resource(DeathAssets { bar_mesh, bar_mat });
}

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_death_assets);
        app.add_systems(
            Update,
            (handle_deaths, cleanup_corpses)
                .chain()
                .in_set(AiSet::Death)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Tag NPCs whose HP hit zero as dead, replace their dot with an X.
fn handle_deaths(
    sim: Option<Res<SimWorld>>,
    death_assets: Res<DeathAssets>,
    mut commands: Commands,
    q: Query<(Entity, &Hp), (With<NpcMarker>, Without<Dead>)>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, hp) in &q {
        if hp.is_alive() {
            continue;
        }
        commands
            .entity(entity)
            .insert(Dead { died_at: now })
            .remove::<Mesh2d>()
            .remove::<MeshMaterial2d<ColorMaterial>>();

        let bar_mesh = death_assets.bar_mesh.clone();
        let bar_mat = death_assets.bar_mat.clone();
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
    q: Query<(Entity, &Dead, &LoadoutComp)>,
) {
    let Some(sim) = sim else { return };
    let now = sim.0.time;

    for (entity, dead, loadout) in &q {
        let elapsed = minutes_between(dead.died_at, now);
        let looted = loadout.0.is_empty();
        if looted || elapsed >= CORPSE_PERSISTENCE_MINUTES {
            commands.entity(entity).despawn();
        }
    }
}

fn to_minutes(t: GameTime) -> u32 {
    (t.day.value() - 1) * 24 * 60 + t.hour as u32 * 60 + t.minute as u32
}

fn minutes_between(start: GameTime, end: GameTime) -> u32 {
    to_minutes(end).saturating_sub(to_minutes(start))
}
