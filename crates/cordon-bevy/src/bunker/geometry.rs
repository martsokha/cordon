//! Low-level geometry spawn helpers: walls, floors, grates, boxes,
//! and the declarative [`PropPlacement`] component that rooms use
//! to ask for prop entities.
//!
//! **Prop placement is data-driven.** Rooms spawn a
//! [`PropPlacement`] component carrying a [`Prop`] variant and a
//! placement; an observer resolves it at `OnAdd` time — loading
//! the scene handle, writing the feet-centre-adjusted
//! `Transform`, and spawning a sibling collider entity if the
//! prop registry says it needs one. Rooms never thread
//! `AssetServer` + `Commands` manually:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use crate::bunker::geometry::{Prop, PropPlacement};
//! # fn sys(mut commands: Commands) {
//! commands.spawn(PropPlacement::new(Prop::Kettle, Vec3::new(0.0, 0.8, 1.0)));
//! commands.spawn(
//!     PropPlacement::new(Prop::Door2, Vec3::new(0.0, 0.0, 5.0))
//!         .rotated(Quat::from_rotation_y(std::f32::consts::PI))
//!         .scaled(1.44),
//! );
//! # }
//! ```

use avian3d::prelude::*;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

pub use super::props::Prop;
use super::resources::RoomCtx;

// Ergonomic shortcuts on RoomCtx for the procedural-geometry
// helpers and prop placement. Rooms prefer `ctx.wall(...)` over
// `spawn_wall(ctx.commands, ctx.meshes, mat.clone(), ...)`, and
// `ctx.prop(Prop::X, pos)` over
// `ctx.commands.spawn(PropPlacement::new(Prop::X, pos))`.
//
// The free functions below stay callable for contexts that
// don't hold a RoomCtx.
impl<'a, 'w, 's> RoomCtx<'a, 'w, 's> {
    /// Place a prop at feet-centre `pos` with no rotation, scale = 1.
    /// Returns the entity's `EntityCommands` so callers can chain
    /// `.id()` or `.insert(marker)` without going through
    /// `ctx.commands.entity(...)`.
    pub fn prop<'c>(&'c mut self, kind: Prop, pos: Vec3) -> EntityCommands<'c> {
        self.commands.spawn(PropPlacement::new(kind, pos))
    }

    /// Place a prop with a rotation.
    pub fn prop_rot<'c>(&'c mut self, kind: Prop, pos: Vec3, rot: Quat) -> EntityCommands<'c> {
        self.commands
            .spawn(PropPlacement::new(kind, pos).rotated(rot))
    }

    /// Place a scaled prop. Rotation + uniform scale in one call.
    pub fn prop_scaled<'c>(
        &'c mut self,
        kind: Prop,
        pos: Vec3,
        rot: Quat,
        scale: f32,
    ) -> EntityCommands<'c> {
        self.commands
            .spawn(PropPlacement::new(kind, pos).rotated(rot).scaled(scale))
    }

    /// Spawn a static wall with visual + collider.
    pub fn wall(
        &mut self,
        center: Vec3,
        rot: Quat,
        half_size: Vec2,
        mat: &Handle<StandardMaterial>,
    ) {
        spawn_wall(
            self.commands,
            self.meshes,
            mat.clone(),
            center,
            rot,
            half_size,
        );
    }

    /// Spawn floor + ceiling planes for a room.
    pub fn floor_ceiling(
        &mut self,
        center: Vec3,
        half_size: Vec2,
        ceiling_y: f32,
        mat: &Handle<StandardMaterial>,
    ) {
        spawn_floor_ceiling(
            self.commands,
            self.meshes,
            mat.clone(),
            center,
            half_size,
            ceiling_y,
        );
    }

    /// Spawn a vertical-bar grate + its full-height collider
    /// slab.
    pub fn grate_bars(
        &mut self,
        x_min: f32,
        x_max: f32,
        z: f32,
        height: f32,
        spacing: f32,
        mat: &Handle<StandardMaterial>,
    ) {
        spawn_grate_bars(
            self.commands,
            self.meshes,
            mat.clone(),
            x_min,
            x_max,
            z,
            height,
            spacing,
        );
    }

    /// Spawn an X-axis doorframe (side pillars + lintel).
    pub fn doorframe_x(
        &mut self,
        x: f32,
        center_z: f32,
        width: f32,
        opening_h: f32,
        mat: &Handle<StandardMaterial>,
    ) {
        spawn_doorframe_x(
            self.commands,
            self.meshes,
            mat.clone(),
            x,
            center_z,
            width,
            opening_h,
        );
    }

    /// Spawn a flight of stairs.
    pub fn stairs(&mut self, start_z: f32, width: f32, steps: u32, mat: &Handle<StandardMaterial>) {
        spawn_stairs(
            self.commands,
            self.meshes,
            mat.clone(),
            start_z,
            width,
            steps,
        );
    }

    /// Spawn a decorative cuboid (no collider). Used for lintels,
    /// counter tops, and other fake-geometry ornaments.
    pub fn decor_box(&mut self, pos: Vec3, size: Vec3, mat: &Handle<StandardMaterial>) {
        spawn_box(self.commands, self.meshes, mat.clone(), pos, size);
    }
}

/// Declarative prop placement. Spawn this as a component and the
/// [`resolve_prop_placement`] observer does the rest: loads the
/// scene handle, applies the feet-centre-adjusted transform,
/// and attaches a collider if the prop's registry entry says so.
///
/// Use the builder methods ([`PropPlacement::rotated`],
/// [`PropPlacement::scaled`]) to tweak orientation / uniform
/// scale. The `pos` is **feet-centre**: lateral AABB centre sits
/// at `(pos.x, pos.z)`, AABB bottom sits at `pos.y`.
#[derive(Component, Debug, Clone, Copy)]
pub struct PropPlacement {
    pub kind: Prop,
    pub pos: Vec3,
    pub rotation: Quat,
    pub scale: f32,
}

impl PropPlacement {
    /// Create a placement for `kind` at feet-centre `pos`. No
    /// rotation, scale = 1.0.
    pub fn new(kind: Prop, pos: Vec3) -> Self {
        Self {
            kind,
            pos,
            rotation: Quat::IDENTITY,
            scale: 1.0,
        }
    }

    /// Apply a local rotation.
    pub fn rotated(mut self, rot: Quat) -> Self {
        self.rotation = rot;
        self
    }

    /// Apply a uniform scale. Feet-centre math stays correct at
    /// any scale.
    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

/// Tag marking a collider entity spawned by
/// [`resolve_prop_placement`] as a sibling of a scene root. Kept
/// so downstream systems can recognise "this collider belongs to
/// a prop" if they ever need to despawn / re-parent.
#[derive(Component)]
pub struct PropCollider;

/// `OnAdd` observer that turns a freshly-spawned
/// [`PropPlacement`] component into a usable prop:
///
/// - Inserts `SceneRoot(handle)` + the feet-centre-adjusted
///   `Transform` on the same entity.
/// - Spawns a sibling collider entity if the prop's AABB is
///   flagged `collider: true`.
///
/// Runs exactly once per placement; the `PropPlacement`
/// component stays on the entity as metadata (cheap, makes the
/// prop's origin rediscoverable).
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

    // Feet-centre → spawn-centre correction (same math the old
    // free `prop()` helper did).
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

    if def.collider {
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

pub fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    pos: Vec3,
    size: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_wall(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    pos: Vec3,
    rot: Quat,
    half_size: Vec2,
) {
    let width = half_size.x * 2.0;
    let height = half_size.y * 2.0;
    let thickness = 0.08;
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(width, height, thickness),
        Mesh3d(meshes.add(Cuboid::new(width, height, thickness))),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).with_rotation(rot),
    ));
}

pub fn spawn_floor_ceiling(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    center: Vec3,
    half_size: Vec2,
    h: f32,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, half_size))),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(center),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Y, half_size))),
        MeshMaterial3d(mat),
        Transform::from_xyz(center.x, h, center.z),
    ));
}

pub fn spawn_grate_bars(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    x_min: f32,
    x_max: f32,
    z: f32,
    height: f32,
    spacing: f32,
) {
    let count = ((x_max - x_min) / spacing) as i32;
    for i in 0..=count {
        let x = x_min + spacing * i as f32;
        if x <= x_max {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.02, height, 0.02))),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, height / 2.0, z),
            ));
        }
    }
    let h_count = (height / 0.4) as i32;
    for i in 1..=h_count {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(x_max - x_min, 0.02, 0.02))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz((x_min + x_max) / 2.0, 0.4 * i as f32, z),
        ));
    }
    let width = x_max - x_min;
    let center_x = (x_min + x_max) / 2.0;
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(width, height, 0.1),
        Transform::from_xyz(center_x, height / 2.0, z),
    ));
}

/// Variant for doorframes facing ±X (opening is in the X direction).
pub fn spawn_doorframe_x(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    x: f32,
    center_z: f32,
    width: f32,
    opening_h: f32,
) {
    let hw = width / 2.0;
    let side_h = opening_h;
    let side_y = side_h / 2.0;
    let lintel_thickness = 0.15;
    let lintel_y = opening_h + lintel_thickness / 2.0;
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(x, side_y, center_z - hw - 0.05),
        Vec3::new(0.15, side_h, 0.1),
    );
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(x, side_y, center_z + hw + 0.05),
        Vec3::new(0.15, side_h, 0.1),
    );
    spawn_box(
        commands,
        meshes,
        mat,
        Vec3::new(x, lintel_y, center_z),
        Vec3::new(0.15, lintel_thickness, width + 0.2),
    );
}

pub fn spawn_stairs(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    start_z: f32,
    width: f32,
    steps: u32,
) {
    for i in 0..steps {
        let step_y = 0.25 * (i + 1) as f32;
        let step_z = start_z + 0.4 * i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(width, step_y, 0.4))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(0.0, step_y / 2.0, step_z),
        ));
    }
}
