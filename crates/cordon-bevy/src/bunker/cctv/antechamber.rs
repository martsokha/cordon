//! The hidden antechamber room the CCTV camera films.
//!
//! Sealed off and placed far below the main bunker geometry
//! (`y = -50`) so the player's main camera frustum never accidentally
//! clips into it. Mirrors the player's room dimensions (4m × 4m,
//! 2.4m tall) so the visitor stands in a familiar-looking space.
//!
//! Geometry only — no behavior. The CCTV camera (in [`super`]) and
//! the visitor sprite (in [`crate::bunker::visitor`]) are spawned
//! elsewhere; this module just builds the walls, door, and lamp.

use bevy::prelude::*;

/// World-space position the antechamber's CCTV camera is aimed at —
/// the visitor sprite stands here while knocking, partway between
/// the back wall (where the door is) and the front wall (where the
/// camera is mounted). The y is half-sprite-height above the floor
/// so they "stand" properly.
pub const ANTECHAMBER_VISITOR_POS: Vec3 = Vec3::new(0.0, -49.75, -49.5);

/// Where the CCTV camera itself sits — front-left ceiling corner
/// of the antechamber, looking diagonally back at the visitor.
pub(super) const CCTV_CAMERA_POS: Vec3 = Vec3::new(-1.85, -47.9, -48.15);

/// Antechamber world centre. The room is built around this point.
const ANTECHAMBER_CENTER: Vec3 = Vec3::new(0.0, -49.0, -50.0);

// Mirror the bunker dimensions: 4m wide × 4m deep × 2.4m tall.
const HALF_W: f32 = 2.0;
const HALF_D: f32 = 2.0;
const HEIGHT: f32 = 2.4;

/// Build the antechamber: floor, four walls, ceiling, the door
/// behind the visitor, a ceiling lamp, and some holding-room
/// furniture.
pub(super) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.17, 0.15),
        perceptual_roughness: 0.92,
        ..default()
    });
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.11, 0.10),
        perceptual_roughness: 0.95,
        ..default()
    });

    let center = ANTECHAMBER_CENTER;
    let hw = HALF_W;
    let hd = HALF_D;
    let h = HEIGHT;

    // Floor.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(hw * 2.0, 0.05, hd * 2.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_translation(center + Vec3::new(0.0, -h / 2.0, 0.0)),
    ));
    // Ceiling.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(hw * 2.0, 0.05, hd * 2.0))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, h / 2.0, 0.0)),
    ));
    // Back wall (-z) — the visitor stands facing +z, so this is
    // *behind* them. The door panel is mounted here.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(hw * 2.0, h, 0.05))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, 0.0, -hd)),
    ));
    // Front wall (+z) — the side the CCTV camera is mounted on.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(hw * 2.0, h, 0.05))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, 0.0, hd)),
    ));
    // Left wall (-x).
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.05, h, hd * 2.0))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(-hw, 0.0, 0.0)),
    ));
    // Right wall (+x).
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.05, h, hd * 2.0))),
        MeshMaterial3d(wall_mat),
        Transform::from_translation(center + Vec3::new(hw, 0.0, 0.0)),
    ));

    spawn_door(commands, meshes, materials, center, hd, h);
    spawn_front_door(commands, meshes, materials, center, hd, h);
    spawn_lamp(commands, center, h);
    spawn_furniture(commands, asset_server, center, hw, hd, h);
}

/// Door on the front wall (+z) — the side facing the bunker
/// interior. Mirrors the back door.
fn spawn_front_door(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    hd: f32,
    h: f32,
) {
    let panel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.06),
        perceptual_roughness: 0.6,
        ..default()
    });
    let frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.20, 0.18),
        perceptual_roughness: 0.5,
        metallic: 0.4,
        ..default()
    });
    let door_w = 0.9;
    let door_h = 2.0;
    let door_z = hd - 0.03;
    let door_y = -h / 2.0 + door_h / 2.0;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(door_w + 0.1, door_h + 0.1, 0.04))),
        MeshMaterial3d(frame_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z - 0.005)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(door_w, door_h, 0.04))),
        MeshMaterial3d(panel_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z - 0.025)),
    ));
}

/// Holding-room furniture: cold, functional, security-checkpoint
/// feel. No comfort — visitors don't get that.
fn spawn_furniture(
    commands: &mut Commands,
    asset_server: &AssetServer,
    center: Vec3,
    hw: f32,
    _hd: f32,
    h: f32,
) {
    use std::f32::consts::FRAC_PI_2;

    use crate::bunker::room::geometry::{Prop, prop};

    let floor = -h / 2.0;

    // Stool — the only seat a visitor gets.
    prop(
        commands,
        asset_server,
        Prop::WoodenStool,
        center + Vec3::new(0.6, floor, -0.5),
        Quat::IDENTITY,
    );
    // Locker against the left wall — for confiscated gear.
    prop(
        commands,
        asset_server,
        Prop::Locker,
        center + Vec3::new(-hw + 0.3, floor, 0.0),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Metal rack on the right wall.
    prop(
        commands,
        asset_server,
        Prop::StorageRack01,
        center + Vec3::new(hw - 0.3, floor, 0.0),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Box on the rack.
    prop(
        commands,
        asset_server,
        Prop::Box02,
        center + Vec3::new(hw - 0.4, floor + 0.6, 0.0),
        Quat::from_rotation_y(0.2),
    );
    // Supply box on the floor.
    prop(
        commands,
        asset_server,
        Prop::Box01,
        center + Vec3::new(-0.8, floor, 0.8),
        Quat::from_rotation_y(0.4),
    );
    // Security panel at eye height.
    prop(
        commands,
        asset_server,
        Prop::ElectricBox01,
        center + Vec3::new(hw - 0.05, 0.0, -0.8),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
}

/// Recessed door panel + slim metallic frame on the back wall,
/// behind where the visitor stands.
fn spawn_door(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    hd: f32,
    h: f32,
) {
    let panel_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.06),
        perceptual_roughness: 0.6,
        ..default()
    });
    let frame_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.20, 0.18),
        perceptual_roughness: 0.5,
        metallic: 0.4,
        ..default()
    });
    let door_w = 0.9;
    let door_h = 2.0;
    let door_z = -hd + 0.03;
    let door_y = -h / 2.0 + door_h / 2.0;

    // Frame: a slightly larger plate behind the panel.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(door_w + 0.1, door_h + 0.1, 0.04))),
        MeshMaterial3d(frame_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z + 0.005)),
    ));
    // Panel: the door itself.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(door_w, door_h, 0.04))),
        MeshMaterial3d(panel_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z + 0.025)),
    ));
}

/// Single warm-but-dim ceiling lamp so the camera has something to
/// see. The antechamber should feel like a holding room, not a
/// lounge.
fn spawn_lamp(commands: &mut Commands, center: Vec3, h: f32) {
    commands.spawn((
        PointLight {
            intensity: 60000.0,
            color: Color::srgb(1.0, 0.85, 0.55),
            range: 8.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(center + Vec3::new(0.0, h / 2.0 - 0.2, 0.0)),
    ));
}
