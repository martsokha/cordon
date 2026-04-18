//! Tiled mesh builders and structural spawn helpers (walls, floors,
//! grates, doorframes, stairs).

use avian3d::prelude::*;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use super::TILE_SIZE;

pub fn spawn_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    pos: Vec3,
    size: Vec3,
    tile: f32,
) {
    commands.spawn((
        Mesh3d(meshes.add(cuboid_tiled(size, tile))),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

/// Build a [`Cuboid`] mesh whose UVs tile proportionally to the
/// cuboid's physical size, keeping texture scale constant across
/// faces of different sizes.
pub fn cuboid_tiled(size: Vec3, tile: f32) -> Mesh {
    let mut mesh = Cuboid::from_size(size).mesh().build();

    let face_scales: [(f32, f32); 6] = [
        (size.x, size.y), // Front  (+Z)
        (size.x, size.y), // Back   (−Z)
        (size.z, size.y), // Right  (+X)
        (size.z, size.y), // Left   (−X)
        (size.x, size.z), // Top    (+Y)
        (size.x, size.z), // Bottom (−Y)
    ];

    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for (face_idx, (scale_u, scale_v)) in face_scales.iter().enumerate() {
            let tiles_u = (scale_u / tile).max(0.0);
            let tiles_v = (scale_v / tile).max(0.0);
            for vert_idx in 0..4 {
                let uv = &mut uvs[face_idx * 4 + vert_idx];
                uv[0] *= tiles_u;
                uv[1] *= tiles_v;
            }
        }
    }

    let _ = mesh.generate_tangents();
    mesh
}

/// Build a [`Plane3d`] mesh whose UVs tile proportionally.
pub fn plane_tiled(normal: Vec3, half_size: Vec2, tile: f32) -> Mesh {
    let mut mesh = Plane3d::new(normal, half_size).mesh().build();
    let size = half_size * 2.0;
    let tiles_u = (size.x / tile).max(0.0);
    let tiles_v = (size.y / tile).max(0.0);
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= tiles_u;
            uv[1] *= tiles_v;
        }
    }
    let _ = mesh.generate_tangents();
    mesh
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
        Mesh3d(meshes.add(cuboid_tiled(Vec3::new(width, height, thickness), TILE_SIZE))),
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
        Mesh3d(meshes.add(plane_tiled(Vec3::Y, half_size, TILE_SIZE))),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(center),
    ));
    commands.spawn((
        Mesh3d(meshes.add(plane_tiled(Vec3::NEG_Y, half_size, TILE_SIZE))),
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
        TILE_SIZE,
    );
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(x, side_y, center_z + hw + 0.05),
        Vec3::new(0.15, side_h, 0.1),
        TILE_SIZE,
    );
    spawn_box(
        commands,
        meshes,
        mat,
        Vec3::new(x, lintel_y, center_z),
        Vec3::new(0.15, lintel_thickness, width + 0.2),
        TILE_SIZE,
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
            Mesh3d(meshes.add(cuboid_tiled(Vec3::new(width, step_y, 0.4), TILE_SIZE))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(0.0, step_y / 2.0, step_z),
        ));
    }
}
