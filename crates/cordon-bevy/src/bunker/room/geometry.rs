//! Low-level geometry spawn helpers: walls, floors, grates, boxes.

use avian3d::prelude::*;
use bevy::prelude::*;

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

pub fn spawn_doorframe(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    center_x: f32,
    z: f32,
    width: f32,
) {
    let hw = width / 2.0;
    spawn_box(commands, meshes, mat.clone(), Vec3::new(center_x - hw - 0.05, 1.05, z), Vec3::new(0.1, 2.1, 0.15));
    spawn_box(commands, meshes, mat.clone(), Vec3::new(center_x + hw + 0.05, 1.05, z), Vec3::new(0.1, 2.1, 0.15));
    spawn_box(commands, meshes, mat, Vec3::new(center_x, 2.15, z), Vec3::new(width + 0.2, 0.15, 0.15));
}

pub fn spawn_doorframe_x(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    x: f32,
    center_z: f32,
    width: f32,
) {
    let hw = width / 2.0;
    spawn_box(commands, meshes, mat.clone(), Vec3::new(x, 1.05, center_z - hw - 0.05), Vec3::new(0.15, 2.1, 0.1));
    spawn_box(commands, meshes, mat.clone(), Vec3::new(x, 1.05, center_z + hw + 0.05), Vec3::new(0.15, 2.1, 0.1));
    spawn_box(commands, meshes, mat, Vec3::new(x, 2.15, center_z), Vec3::new(0.15, 0.15, width + 0.2));
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

/// Spawn a GLB scene at a given position and rotation.
pub fn glb(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: &str,
    pos: Vec3,
    rot: Quat,
) {
    let scene: Handle<Scene> = asset_server.load(format!("{path}#Scene0"));
    commands.spawn((
        SceneRoot(scene),
        Transform::from_translation(pos).with_rotation(rot),
    ));
}
