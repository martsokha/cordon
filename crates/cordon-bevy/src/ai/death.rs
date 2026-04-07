//! Visual reaction to NPC deaths.
//!
//! The sim layer ([`cordon_sim::death`]) tags entities with `Dead` and
//! emits `NpcDied`. This module reads those events and replaces the
//! NPC's round dot mesh with an X marker. Despawn happens sim-side
//! when the corpse is fully looted or expires; the X mesh children
//! follow the parent entity into oblivion.

use bevy::prelude::*;
use cordon_sim::events::NpcDied;
use cordon_sim::plugin::SimSet;

use crate::PlayingState;

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

pub struct DeathVisualsPlugin;

impl Plugin for DeathVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_death_assets);
        app.add_systems(
            Update,
            apply_death_visuals
                .after(SimSet::Death)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// React to `NpcDied`: drop the dot mesh and stamp on a crossed-bar X
/// as two child entities.
fn apply_death_visuals(
    mut commands: Commands,
    mut deaths: MessageReader<NpcDied>,
    death_assets: Res<DeathAssets>,
) {
    for ev in deaths.read() {
        let Ok(mut e) = commands.get_entity(ev.entity) else {
            continue;
        };
        e.remove::<Mesh2d>()
            .remove::<MeshMaterial2d<ColorMaterial>>();

        let bar_mesh = death_assets.bar_mesh.clone();
        let bar_mat = death_assets.bar_mat.clone();
        e.with_children(|parent| {
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
