//! Declarative prop placement via the [`PropPlacement`] component
//! and its `OnAdd` observer.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::Prop;

/// Declarative prop placement. Spawn this as a component and the
/// [`resolve_prop_placement`] observer does the rest: loads the
/// scene handle, applies the feet-centre-adjusted transform,
/// and attaches a collider if the prop's registry entry says so.
#[derive(Component, Debug, Clone, Copy)]
pub struct PropPlacement {
    pub kind: Prop,
    pub pos: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub no_collider: bool,
}

impl PropPlacement {
    pub fn new(kind: Prop, pos: Vec3) -> Self {
        Self {
            kind,
            pos,
            rotation: Quat::IDENTITY,
            scale: 1.0,
            no_collider: false,
        }
    }

    pub fn rotated(mut self, rot: Quat) -> Self {
        self.rotation = rot;
        self
    }

    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn no_collider(mut self) -> Self {
        self.no_collider = true;
        self
    }
}

/// Tag marking a collider entity spawned by
/// [`resolve_prop_placement`] as a sibling of a scene root.
#[derive(Component)]
pub struct PropCollider;

/// `OnAdd` observer that turns a [`PropPlacement`] component into
/// a usable prop entity with scene root, transform, and collider.
pub fn resolve_prop_placement(
    trigger: On<Add, PropPlacement>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placements: Query<&PropPlacement>,
) {
    let entity = trigger.event().entity;
    let Ok(p) = placements.get(entity) else {
        return;
    };
    let def = p.kind.def();

    let local_center = (def.aabb_min + def.aabb_max) * 0.5 * p.scale;
    let feet_local = Vec3::new(local_center.x, def.aabb_min.y * p.scale, local_center.z);
    let spawn_pos = p.pos - p.rotation * feet_local;

    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", def.path));
    commands.entity(entity).insert((
        SceneRoot(scene),
        Transform::from_translation(spawn_pos)
            .with_rotation(p.rotation)
            .with_scale(Vec3::splat(p.scale)),
    ));

    if def.collider && !p.no_collider {
        let size = (def.aabb_max - def.aabb_min) * p.scale;
        let collider_center = spawn_pos + p.rotation * local_center;
        commands.spawn((
            PropCollider,
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            Transform::from_translation(collider_center).with_rotation(p.rotation),
        ));
    }
}
