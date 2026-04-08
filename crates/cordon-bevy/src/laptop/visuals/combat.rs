//! Visual reaction to weapon fire.
//!
//! The sim layer ([`cordon_sim::combat`]) resolves combat and emits
//! `ShotFired` for each successful shot. This module spawns a tracer
//! line entity per event and fades it out.

use bevy::prelude::*;
use cordon_sim::combat::ShotFired;
use cordon_sim::plugin::SimSet;

use crate::PlayingState;
use crate::laptop::map::MapWorldEntity;

const TRACER_LIFE_SECS: f32 = 0.18;
const TRACER_WIDTH: f32 = 0.7;
const TRACER_COLOR: Color = Color::srgba(1.0, 0.92, 0.55, 0.95);

/// A short-lived line drawn from shooter to target on each shot.
#[derive(Component, Debug, Clone, Copy)]
pub struct Tracer {
    pub life_secs: f32,
}

/// Shared mesh + material for tracer entities.
#[derive(Resource, Clone)]
pub struct TracerAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

fn init_tracer_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, TRACER_WIDTH));
    let material = materials.add(ColorMaterial::from_color(TRACER_COLOR));
    commands.insert_resource(TracerAssets { mesh, material });
}

pub struct CombatVisualsPlugin;

impl Plugin for CombatVisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_tracer_assets);
        app.add_systems(
            Update,
            (spawn_tracers_for_shots.after(SimSet::Combat), fade_tracers)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

fn spawn_tracers_for_shots(
    mut commands: Commands,
    assets: Res<TracerAssets>,
    mut shots: MessageReader<ShotFired>,
) {
    for ev in shots.read() {
        let delta = ev.to - ev.from;
        let length = delta.length();
        if length < 0.5 {
            continue;
        }
        let mid = (ev.from + ev.to) * 0.5;
        let angle = delta.y.atan2(delta.x);
        commands.spawn((
            MapWorldEntity,
            Tracer {
                life_secs: TRACER_LIFE_SECS,
            },
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(assets.material.clone()),
            Transform {
                translation: Vec3::new(mid.x, mid.y, 0.6),
                rotation: Quat::from_rotation_z(angle),
                scale: Vec3::new(length, 1.0, 1.0),
            },
        ));
    }
}

fn fade_tracers(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut Tracer)>) {
    let dt = time.delta_secs();
    for (entity, mut tracer) in &mut q {
        tracer.life_secs -= dt;
        if tracer.life_secs <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
