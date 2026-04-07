//! Visual reaction to NPC deaths.
//!
//! The sim layer ([`cordon_sim::death`]) tags entities with `Dead` and
//! emits `NpcDied`. This module reads those events and replaces the
//! NPC's round dot mesh with an X marker. Despawn happens sim-side
//! when the corpse is fully looted or expires; the X mesh children
//! follow the parent entity into oblivion.

use bevy::prelude::*;
use cordon_sim::components::{FactionId, NpcMarker};
use cordon_sim::events::NpcDied;
use cordon_sim::plugin::SimSet;

use crate::AppState;
use crate::laptop::FactionPalette;

const X_BAR_LENGTH: f32 = 10.0;
const X_BAR_THICKNESS: f32 = 1.5;
const X_BAR_FALLBACK_COLOR: Color = Color::srgba(0.55, 0.55, 0.55, 0.9);

#[derive(Resource, Clone)]
pub struct DeathAssets {
    pub bar_mesh: Handle<Mesh>,
    pub fallback_mat: Handle<ColorMaterial>,
}

fn init_death_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let bar_mesh = meshes.add(Rectangle::new(X_BAR_LENGTH, X_BAR_THICKNESS));
    let fallback_mat = materials.add(ColorMaterial::from_color(X_BAR_FALLBACK_COLOR));
    commands.insert_resource(DeathAssets {
        bar_mesh,
        fallback_mat,
    });
}

pub struct DeathVisualsPlugin;

impl Plugin for DeathVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_death_assets);
        // Run while the world is playing, not only when the laptop
        // is open. `NpcDied` is a one-shot message — if we drained it
        // here only in laptop state, NPCs that died in the bunker
        // would keep their alive-dot visuals forever.
        app.add_systems(
            Update,
            apply_death_visuals
                .after(SimSet::Death)
                .run_if(in_state(AppState::Playing)),
        );
    }
}

/// React to `NpcDied`: drop the dot mesh and stamp on a crossed-bar X
/// as two child entities, tinted by the dead NPC's faction so the
/// corpse is recognizable on the map.
fn apply_death_visuals(
    mut commands: Commands,
    mut deaths: MessageReader<NpcDied>,
    death_assets: Res<DeathAssets>,
    palette: Res<FactionPalette>,
    factions: Query<&FactionId, With<NpcMarker>>,
) {
    for ev in deaths.read() {
        // The faction lookup may legitimately fail if the entity was
        // already despawned this frame; fall back to the neutral X
        // material in that case.
        let bar_mat = match factions.get(ev.entity) {
            Ok(f) => palette.corpse(&f.0),
            Err(_) => death_assets.fallback_mat.clone(),
        };

        let Ok(mut e) = commands.get_entity(ev.entity) else {
            continue;
        };
        e.remove::<Mesh2d>()
            .remove::<MeshMaterial2d<ColorMaterial>>();

        let bar_mesh = death_assets.bar_mesh.clone();
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
