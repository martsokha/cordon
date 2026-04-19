use std::f32::consts::{FRAC_PI_2, PI};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::components::CctvMonitor;
use super::materials::CctvMaterial;
use crate::bunker::geometry::{Prop, PropPlacement};

/// Half-width / half-height of the render-target plane, sized to
/// fit inside the `Monitor_02` visible glass with a little bezel
/// margin.
const SCREEN_HALF_EXTENT: Vec2 = Vec2::new(0.25, 0.20);

/// Distance from the wall-mount anchor forward along the
/// monitor's facing direction to the bezel origin.
/// `Monitor_02`'s body trails 0.33 m behind its origin on
/// local -Z, so we push the origin forward by that depth to
/// keep the body flush with the wall instead of poking
/// through it.
const BEZEL_FORWARD_OFFSET: f32 = 0.33;

/// Approximate height of the centre of the CRT's glass above the
/// model's origin (Y range is 0..0.6).
const SCREEN_CENTER_HEIGHT: f32 = 0.32;

/// Distance from the bezel origin to the screen plane along the
/// bezel's facing direction. Sits just outside the front glass.
const SCREEN_FORWARD_OFFSET: f32 = 0.28;

/// How far the centre of the curved screen bulges out from a
/// flat plane, in metres. Readable as "old CRT glass" without
/// tipping into disco-ball territory.
const SCREEN_BULGE: f32 = 0.06;

/// Grid subdivisions per axis for the curved screen mesh. Needs
/// enough verts that the bulge reads smooth, not enough to waste
/// draw calls on a tiny surface.
const SCREEN_SUBDIV: u32 = 16;

pub fn spawn_monitor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    _std_materials: &mut Assets<StandardMaterial>,
    cctv_materials: &mut Assets<CctvMaterial>,
    image: Handle<Image>,
    monitor_pos: Vec3,
    monitor_target: Vec3,
) -> Entity {
    let dir = (monitor_target - monitor_pos).normalize_or_zero();

    // Bezel: `Monitor_02` has its screen on local +Z and its
    // body trailing back on -Z. Compose:
    //   looking_at(target)        — local -Z points at target
    //   * rotation_y(PI)          — flip so local +Z points at target
    //   * rotation_y(-FRAC_PI_2)  — yaw 90° CW around the monitor's
    //                                own vertical axis
    let bezel_rot = Transform::default()
        .looking_at(monitor_target - monitor_pos, Vec3::Y)
        .rotation
        * Quat::from_rotation_y(PI)
        * Quat::from_rotation_y(-FRAC_PI_2);
    let bezel_pos = monitor_pos + dir * BEZEL_FORWARD_OFFSET;
    commands.spawn(PropPlacement::new(Prop::Monitor02, bezel_pos).rotated(bezel_rot));

    // Screen plane: inherits the bezel's rotation, then yaws an
    // extra 90° CCW around its own vertical so the surface faces
    // the player instead of sideways. Positioned along the
    // bezel's local +Z (forward) and lifted to the CRT screen
    // centre via the bezel's local +Y (up).
    let screen_mat = cctv_materials.add(CctvMaterial::new(image, 1.0));
    let plane_rot = bezel_rot * Quat::from_rotation_y(FRAC_PI_2);
    let local_forward = plane_rot * Vec3::Z;
    let local_up = plane_rot * Vec3::Y;
    let screen_pos =
        bezel_pos + local_forward * SCREEN_FORWARD_OFFSET + local_up * SCREEN_CENTER_HEIGHT;
    let monitor_transform = Transform::from_translation(screen_pos).with_rotation(plane_rot);
    commands
        .spawn((
            CctvMonitor,
            Mesh3d(meshes.add(curved_screen_mesh(
                SCREEN_HALF_EXTENT,
                SCREEN_BULGE,
                SCREEN_SUBDIV,
            ))),
            MeshMaterial3d(screen_mat),
            monitor_transform,
        ))
        .id()
}

/// Build a subdivided plane on the XY axes with its centre
/// displaced along +Z to approximate a CRT's convex glass.
/// Displacement follows `bulge * (1 - u^2 - v^2)` clamped to
/// zero so corners sit flat while the middle pokes out. UVs are
/// laid out `[0,1] × [0,1]` so any texture sampling it sees the
/// same distribution it would on a `Plane3d`.
fn curved_screen_mesh(half_extent: Vec2, bulge: f32, subdiv: u32) -> Mesh {
    let n = subdiv.max(1) as usize;
    let verts_per_side = n + 1;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(verts_per_side * verts_per_side);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_per_side * verts_per_side);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(verts_per_side * verts_per_side);

    for j in 0..verts_per_side {
        let v = j as f32 / n as f32;
        let y = (v - 0.5) * 2.0 * half_extent.y;
        let vn = (v - 0.5) * 2.0;
        for i in 0..verts_per_side {
            let u = i as f32 / n as f32;
            let x = (u - 0.5) * 2.0 * half_extent.x;
            let un = (u - 0.5) * 2.0;

            let r2 = un * un + vn * vn;
            let z = (bulge * (1.0 - r2)).max(0.0);

            // Analytic normal of `z = b*(1 - (x/w)^2 - (y/h)^2)`:
            // grad z = (-2b*un / w_world, -2b*vn / h_world), so
            // normal ∝ (-dz/dx, -dz/dy, 1). The exact world-space
            // slope uses the actual half-extents, not the
            // normalised `un`/`vn`.
            let dzdx = -2.0 * bulge * un / half_extent.x;
            let dzdy = -2.0 * bulge * vn / half_extent.y;
            let n_vec = Vec3::new(-dzdx, -dzdy, 1.0).normalize();

            positions.push([x, y, z]);
            uvs.push([u, 1.0 - v]);
            normals.push(n_vec.to_array());
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity(n * n * 6);
    for j in 0..n {
        for i in 0..n {
            let a = (j * verts_per_side + i) as u32;
            let b = a + 1;
            let c = a + verts_per_side as u32;
            let d = c + 1;
            // Two triangles per quad, wound CCW when viewed from
            // +Z so the front face matches the player-facing
            // orientation `Plane3d::new(Vec3::Z, …)` produces.
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
