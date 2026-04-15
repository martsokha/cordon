//! The hidden antechamber room the CCTV camera films.
//!
//! Sealed off and placed far below the main bunker geometry
//! (`y = -50`) so the player's main camera frustum never
//! accidentally clips into it. Mirrors the player's room
//! dimensions (4m × 4m, 2.4m tall) so the visitor stands in a
//! familiar-looking space.
//!
//! Geometry only — no behavior. The CCTV camera (in [`super`])
//! and the visitor sprite (in [`crate::bunker::visitor`]) are
//! spawned elsewhere; this module just builds the walls, door,
//! and lamp.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::bunker::geometry::Prop;
use crate::bunker::resources::RoomCtx;

/// Antechamber world centre. The room is built around this point.
const ANTECHAMBER_CENTER: Vec3 = Vec3::new(0.0, -49.0, -50.0);

// Mirror the bunker dimensions: 4m wide × 4m deep × 2.4m tall.
const HALF_W: f32 = 2.0;
const HALF_D: f32 = 2.0;
const HEIGHT: f32 = 2.4;

/// Convert a position expressed in the antechamber's local frame —
/// where (0, 0, 0) is the center of the floor — into world space.
/// Lets callers write placements as if the room started at the
/// origin, without having to remember the y = -50 offset.
fn local_to_world(local: Vec3) -> Vec3 {
    ANTECHAMBER_CENTER + Vec3::new(local.x, local.y - HEIGHT / 2.0, local.z)
}

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let wall_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.17, 0.15),
        perceptual_roughness: 0.92,
        ..default()
    });
    let floor_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.11, 0.10),
        perceptual_roughness: 0.95,
        ..default()
    });

    let center = ANTECHAMBER_CENTER;
    let hw = HALF_W;
    let hd = HALF_D;
    let h = HEIGHT;

    // Walls, floor, ceiling. Built as raw Cuboid meshes rather
    // than going through `ctx.wall` because the antechamber
    // uses custom wall thickness (0.05 m slabs) and its own
    // material palette distinct from the main bunker's.
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(hw * 2.0, 0.05, hd * 2.0))),
        MeshMaterial3d(floor_mat),
        Transform::from_translation(center + Vec3::new(0.0, -h / 2.0, 0.0)),
    ));
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(hw * 2.0, 0.05, hd * 2.0))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, h / 2.0, 0.0)),
    ));
    // Back wall (-z) — the visitor stands facing +z, so this is
    // *behind* them. The door panel is mounted here.
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(hw * 2.0, h, 0.05))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, 0.0, -hd)),
    ));
    // Front wall (+z) — the side the CCTV camera is mounted on.
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(hw * 2.0, h, 0.05))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(0.0, 0.0, hd)),
    ));
    // Left wall (-x).
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(0.05, h, hd * 2.0))),
        MeshMaterial3d(wall_mat.clone()),
        Transform::from_translation(center + Vec3::new(-hw, 0.0, 0.0)),
    ));
    // Right wall (+x).
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(0.05, h, hd * 2.0))),
        MeshMaterial3d(wall_mat),
        Transform::from_translation(center + Vec3::new(hw, 0.0, 0.0)),
    ));

    // Back door: real GLB prop (same model + scale as the
    // bunker entrance). Stands against the back wall (-z),
    // facing the visitor so they "came through" it.
    const DOOR_SCALE: f32 = 1.44;
    ctx.prop_scaled(
        Prop::Door2,
        local_to_world(Vec3::new(0.0, 0.0, -hd + 0.12)),
        Quat::IDENTITY,
        DOOR_SCALE,
    );
    spawn_front_door(ctx, center, hd, h);
    spawn_lamp(ctx, center, h);
    spawn_furniture(ctx, hw, hd);
}

/// Door on the front wall (+z) — the side facing the bunker
/// interior. Mirrors the back door.
fn spawn_front_door(ctx: &mut RoomCtx<'_, '_, '_>, center: Vec3, hd: f32, h: f32) {
    let panel_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.06),
        perceptual_roughness: 0.6,
        ..default()
    });
    let frame_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.20, 0.18),
        perceptual_roughness: 0.5,
        metallic: 0.4,
        ..default()
    });
    let door_w = 0.9;
    let door_h = 2.0;
    let door_z = hd - 0.03;
    let door_y = -h / 2.0 + door_h / 2.0;

    ctx.commands.spawn((
        Mesh3d(
            ctx.meshes
                .add(Cuboid::new(door_w + 0.1, door_h + 0.1, 0.04)),
        ),
        MeshMaterial3d(frame_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z - 0.005)),
    ));
    ctx.commands.spawn((
        Mesh3d(ctx.meshes.add(Cuboid::new(door_w, door_h, 0.04))),
        MeshMaterial3d(panel_mat),
        Transform::from_translation(center + Vec3::new(0.0, door_y, door_z - 0.025)),
    ));
}

/// Holding-room furniture: cold, functional, security-checkpoint
/// feel. No comfort — visitors don't get that.
fn spawn_furniture(ctx: &mut RoomCtx<'_, '_, '_>, hw: f32, _hd: f32) {
    // Stool — the only seat a visitor gets.
    ctx.prop(Prop::WoodenStool, local_to_world(Vec3::new(0.6, 0.0, -0.5)));
    // Locker against the left wall — for confiscated gear.
    ctx.prop_rot(
        Prop::Locker,
        local_to_world(Vec3::new(-hw + 0.3, 0.0, 0.0)),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Box against the right wall. Used to sit on a rack shelf,
    // but the rack was removed to keep the antechamber sparse.
    ctx.prop_rot(
        Prop::Box02,
        local_to_world(Vec3::new(hw - 0.4, 0.0, 0.0)),
        Quat::from_rotation_y(0.2),
    );
    // Supply box on the floor.
    ctx.prop_rot(
        Prop::Box01,
        local_to_world(Vec3::new(-0.8, 0.0, 0.8)),
        Quat::from_rotation_y(0.4),
    );
    // Security panel mounted mid-wall.
    ctx.prop_rot(
        Prop::ElectricBox01,
        local_to_world(Vec3::new(hw - 0.05, HEIGHT / 2.0, -0.8)),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
}

/// Single warm-but-dim ceiling lamp so the camera has something
/// to see. The antechamber should feel like a holding room, not
/// a lounge.
fn spawn_lamp(ctx: &mut RoomCtx<'_, '_, '_>, center: Vec3, h: f32) {
    ctx.commands.spawn((
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
