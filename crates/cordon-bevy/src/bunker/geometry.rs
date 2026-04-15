//! Low-level geometry spawn helpers: walls, floors, grates, boxes.

use avian3d::prelude::*;
use bevy::prelude::*;

pub use super::props::Prop;

/// Spawn a registered prop at `pos`, where `pos` is the **feet-center**:
/// the model's lateral AABB center sits at `pos.x`/`pos.z` and its
/// lowest point sits at `pos.y`. Set `pos.y = 0.0` for floor props, or
/// the shelf surface height for things that sit on a shelf.
///
/// If the prop's registry entry has `collider = true`, a sibling static
/// collider is spawned matching the GLB's measured AABB. Rotation
/// (around Y — which is all the rooms use) applies to both.
/// Returns the spawned prop entity so callers can parent child
/// effects (e.g. particle emitters) to it.
pub fn prop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    prop: Prop,
    pos: Vec3,
    rot: Quat,
) -> Entity {
    prop_scaled(commands, asset_server, prop, pos, rot, 1.0)
}

/// Uniformly-scaled variant of [`prop`]. Scales the mesh, the
/// feet-center offset math, and the collider so the scaled prop
/// still sits flush on `pos.y`.
pub fn prop_scaled(
    commands: &mut Commands,
    asset_server: &AssetServer,
    prop: Prop,
    pos: Vec3,
    rot: Quat,
    scale: f32,
) -> Entity {
    let def = prop.def();
    let size = (def.aabb_max - def.aabb_min) * scale;
    let local_center = (def.aabb_min + def.aabb_max) * 0.5 * scale;
    let feet_local = Vec3::new(local_center.x, def.aabb_min.y * scale, local_center.z);
    let spawn_pos = pos - rot * feet_local;

    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", def.path));
    let entity = commands
        .spawn((
            SceneRoot(scene),
            Transform::from_translation(spawn_pos)
                .with_rotation(rot)
                .with_scale(Vec3::splat(scale)),
        ))
        .id();

    if def.collider {
        let collider_center = spawn_pos + rot * local_center;
        commands.spawn((
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            Transform::from_translation(collider_center).with_rotation(rot),
        ));
    }

    entity
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
