//! Blockout geometry: reusable building blocks for the bunker scene.

use bevy::prelude::*;

use super::{BunkerSpawned, BunkerUi, FpsCamera, InteractPrompt, LaptopObject};
use crate::PlayingState;

pub struct BlockoutPlugin;

impl Plugin for BlockoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            spawn_bunker.run_if(not(resource_exists::<BunkerSpawned>)),
        );
    }
}

/// Shared materials for blockout geometry.
struct Palette {
    concrete: Handle<StandardMaterial>,
    concrete_dark: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    metal_dark: Handle<StandardMaterial>,
    crate_: Handle<StandardMaterial>,
}

impl Palette {
    fn new(mats: &mut Assets<StandardMaterial>) -> Self {
        Self {
            concrete: mats.add(StandardMaterial {
                base_color: Color::srgb(0.14, 0.13, 0.12),
                perceptual_roughness: 0.95,
                ..default()
            }),
            concrete_dark: mats.add(StandardMaterial {
                base_color: Color::srgb(0.10, 0.10, 0.09),
                perceptual_roughness: 0.95,
                ..default()
            }),
            wood: mats.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.16, 0.10),
                perceptual_roughness: 0.85,
                ..default()
            }),
            metal: mats.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.28, 0.26),
                perceptual_roughness: 0.5,
                metallic: 0.6,
                ..default()
            }),
            metal_dark: mats.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.22, 0.21),
                perceptual_roughness: 0.7,
                metallic: 0.4,
                ..default()
            }),
            crate_: mats.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.18, 0.12),
                perceptual_roughness: 0.9,
                ..default()
            }),
        }
    }
}

// === Reusable spawn helpers ===

fn spawn_box(
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

fn spawn_wall(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    pos: Vec3,
    rot: Quat,
    half_size: Vec2,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, half_size))),
        MeshMaterial3d(mat),
        Transform::from_translation(pos).with_rotation(rot),
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_grate_bars(
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
    // Horizontal bars
    let h_count = (height / 0.4) as i32;
    for i in 1..=h_count {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(x_max - x_min, 0.02, 0.02))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz((x_min + x_max) / 2.0, 0.4 * i as f32, z),
        ));
    }
}

fn spawn_shelf_unit(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    center: Vec3,
    width: f32,
    depth: f32,
    tiers: u32,
) {
    let tier_h = 1.8 / tiers as f32;
    for i in 1..=tiers {
        let y = tier_h * i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(width, 0.02, depth))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(center.x, y, center.z),
        ));
    }
    // Uprights
    for dx in [-width / 2.0 + 0.02, width / 2.0 - 0.02] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.03, 1.8, 0.03))),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(center.x + dx, 0.9, center.z),
        ));
    }
}

fn spawn_locker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    pos: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.45, 1.8, 0.4))),
        MeshMaterial3d(mat),
        Transform::from_xyz(pos.x, 0.9, pos.z),
    ));
}

fn spawn_desk_enclosed(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    center: Vec3,
    width: f32,
    depth: f32,
) {
    let hw = width / 2.0;
    let hd = depth / 2.0;
    // Top
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(center.x, 0.75, center.z),
        Vec3::new(width, 0.05, depth),
    );
    // Front panel
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(center.x, 0.375, center.z + hd),
        Vec3::new(width, 0.75, 0.03),
    );
    // Left panel
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(center.x - hw, 0.375, center.z),
        Vec3::new(0.03, 0.75, depth),
    );
    // Right panel
    spawn_box(
        commands,
        meshes,
        mat,
        Vec3::new(center.x + hw, 0.375, center.z),
        Vec3::new(0.03, 0.75, depth),
    );
}

fn spawn_doorframe(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: Handle<StandardMaterial>,
    center_x: f32,
    z: f32,
    width: f32,
) {
    let hw = width / 2.0;
    // Left jamb
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(center_x - hw - 0.05, 1.05, z),
        Vec3::new(0.1, 2.1, 0.15),
    );
    // Right jamb
    spawn_box(
        commands,
        meshes,
        mat.clone(),
        Vec3::new(center_x + hw + 0.05, 1.05, z),
        Vec3::new(0.1, 2.1, 0.15),
    );
    // Lintel
    spawn_box(
        commands,
        meshes,
        mat,
        Vec3::new(center_x, 2.15, z),
        Vec3::new(width + 0.2, 0.15, 0.15),
    );
}

fn spawn_stairs(
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

// === Main bunker spawn ===

fn spawn_bunker(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    use std::f32::consts::{FRAC_PI_2, PI};

    let pal = Palette::new(&mut mats);

    let h = 2.4;
    let hw = 2.0;
    let back_z = -5.0;
    let front_z = 5.0;
    let trade_z = 1.5;
    let divider_z = -1.5;
    let desk_z = trade_z - 0.5;
    let hole_half = 0.6;

    // Camera
    commands.spawn((
        FpsCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.6, desk_z - 0.5)
            .looking_at(Vec3::new(0.0, 1.2, front_z), Vec3::Y),
    ));

    // Lighting
    for (pos, intensity, color) in [
        (
            Vec3::new(0.0, h - 0.15, desk_z),
            80000.0,
            Color::srgb(1.0, 0.88, 0.6),
        ),
        (
            Vec3::new(0.0, h - 0.15, -3.0),
            40000.0,
            Color::srgb(0.95, 0.8, 0.55),
        ),
        (
            Vec3::new(0.0, h - 0.15, 3.0),
            30000.0,
            Color::srgb(0.9, 0.75, 0.5),
        ),
    ] {
        commands.spawn((
            PointLight {
                intensity,
                color,
                range: 10.0,
                shadows_enabled: pos.z == desk_z,
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }
    // Monitor glow
    commands.spawn((
        PointLight {
            intensity: 6000.0,
            color: Color::srgb(0.5, 0.8, 0.5),
            range: 2.5,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 1.1, desk_z),
    ));

    // Floor + ceiling
    let center_z = (front_z + back_z) / 2.0;
    let floor_half = Vec2::new(hw, (front_z - back_z) / 2.0);
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, floor_half))),
        MeshMaterial3d(pal.concrete_dark.clone()),
        Transform::from_xyz(0.0, 0.0, center_z),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Y, floor_half))),
        MeshMaterial3d(pal.concrete_dark.clone()),
        Transform::from_xyz(0.0, h, center_z),
    ));

    // Walls
    let hh = h / 2.0;
    let dh = (front_z - back_z) / 2.0;
    for (pos, rot, size) in [
        (
            Vec3::new(0.0, hh, back_z),
            Quat::IDENTITY,
            Vec2::new(hw, hh),
        ),
        (
            Vec3::new(0.0, hh, front_z),
            Quat::from_rotation_y(PI),
            Vec2::new(hw, hh),
        ),
        (
            Vec3::new(-hw, hh, center_z),
            Quat::from_rotation_y(FRAC_PI_2),
            Vec2::new(dh, hh),
        ),
        (
            Vec3::new(hw, hh, center_z),
            Quat::from_rotation_y(-FRAC_PI_2),
            Vec2::new(dh, hh),
        ),
    ] {
        spawn_wall(
            &mut commands,
            &mut meshes,
            pal.concrete.clone(),
            pos,
            rot,
            size,
        );
    }

    // Back door
    spawn_box(
        &mut commands,
        &mut meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 1.0, back_z + 0.05),
        Vec3::new(0.9, 2.0, 0.08),
    );

    // Internal divider grate (left and right, open center)
    spawn_grate_bars(
        &mut commands,
        &mut meshes,
        pal.metal.clone(),
        -hw,
        -hole_half,
        divider_z,
        h,
        0.12,
    );
    spawn_grate_bars(
        &mut commands,
        &mut meshes,
        pal.metal.clone(),
        hole_half,
        hw,
        divider_z,
        h,
        0.12,
    );

    // === STORAGE (back_z to divider_z) ===
    for z in [-2.5, -4.0] {
        spawn_shelf_unit(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(-hw + 0.3, 0.0, z),
            0.5,
            1.2,
            3,
        );
    }
    for z in [-2.5, -4.0] {
        spawn_shelf_unit(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(hw - 0.3, 0.0, z),
            0.5,
            1.2,
            3,
        );
    }
    // Crates
    for (pos, size) in [
        (Vec3::new(0.5, 0.15, -4.0), Vec3::new(0.3, 0.3, 0.3)),
        (Vec3::new(-0.3, 0.15, -4.3), Vec3::new(0.4, 0.3, 0.25)),
        (Vec3::new(1.0, 0.2, -2.5), Vec3::new(0.35, 0.4, 0.35)),
        (Vec3::new(-0.8, 0.15, -2.0), Vec3::new(0.25, 0.25, 0.3)),
    ] {
        spawn_box(&mut commands, &mut meshes, pal.crate_.clone(), pos, size);
    }

    // === DESK AREA (divider_z to trade_z) ===
    spawn_desk_enclosed(
        &mut commands,
        &mut meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 0.0, desk_z),
        1.4,
        0.6,
    );

    // Chair behind desk
    // Seat
    spawn_box(
        &mut commands,
        &mut meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 0.45, desk_z - 0.5),
        Vec3::new(0.4, 0.04, 0.4),
    );
    // Backrest
    spawn_box(
        &mut commands,
        &mut meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 0.7, desk_z - 0.7),
        Vec3::new(0.4, 0.5, 0.04),
    );
    // Legs
    for (dx, dz) in [(-0.17, -0.17), (0.17, -0.17), (-0.17, 0.17), (0.17, 0.17)] {
        spawn_box(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(dx, 0.225, desk_z - 0.5 + dz),
            Vec3::new(0.03, 0.45, 0.03),
        );
    }

    // Shelves in desk room (both walls, two per side)
    for z in [-0.6, 0.8] {
        spawn_shelf_unit(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(-hw + 0.3, 0.0, z),
            0.5,
            1.2,
            3,
        );
    }
    for z in [-0.6, 0.8] {
        spawn_shelf_unit(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(hw - 0.3, 0.0, z),
            0.5,
            1.2,
            3,
        );
    }

    // Laptop
    spawn_box(
        &mut commands,
        &mut meshes,
        mats.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.08),
            perceptual_roughness: 0.8,
            ..default()
        }),
        Vec3::new(0.0, 0.79, desk_z),
        Vec3::new(0.36, 0.02, 0.25),
    );

    commands.spawn((
        LaptopObject,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.17, 0.12)))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.12, 0.08),
            emissive: LinearRgba::new(0.08, 0.15, 0.08, 1.0),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.92, desk_z + 0.12).with_rotation(Quat::from_rotation_x(-0.35)),
    ));

    // === TRADE GRATE (z = trade_z) ===
    // Left grate
    spawn_grate_bars(
        &mut commands,
        &mut meshes,
        pal.metal.clone(),
        -hw,
        -hole_half,
        trade_z,
        h,
        0.1,
    );
    // Right grate
    spawn_grate_bars(
        &mut commands,
        &mut meshes,
        pal.metal.clone(),
        hole_half,
        hw,
        trade_z,
        h,
        0.1,
    );
    // Counter shelf at opening
    spawn_box(
        &mut commands,
        &mut meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 0.78, trade_z),
        Vec3::new(hole_half * 2.0 + 0.2, 0.04, 0.25),
    );

    // === VISITOR SIDE (z > trade_z) ===

    // Lockers on LEFT side (5 together)
    for i in 0..5 {
        spawn_locker(
            &mut commands,
            &mut meshes,
            pal.metal_dark.clone(),
            Vec3::new(-hw + 0.3, 0.0, 2.2 + 0.5 * i as f32),
        );
    }

    // Wide container on RIGHT side
    spawn_box(
        &mut commands,
        &mut meshes,
        mats.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.18, 0.17),
            perceptual_roughness: 0.6,
            metallic: 0.5,
            ..default()
        }),
        Vec3::new(hw - 0.4, 0.3, 3.0),
        Vec3::new(0.7, 0.6, 1.0),
    );

    // Open doorframe to stairs
    spawn_doorframe(
        &mut commands,
        &mut meshes,
        pal.concrete.clone(),
        0.0,
        front_z - 0.1,
        1.0,
    );

    // Stairs ascending away
    spawn_stairs(
        &mut commands,
        &mut meshes,
        pal.concrete.clone(),
        front_z + 0.3,
        1.0,
        6,
    );

    // === UI ===
    commands.spawn((
        BunkerUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(49.0),
            top: Val::Percent(48.0),
            ..default()
        },
        Text::new("+"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
    ));

    commands.spawn((
        InteractPrompt,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(55.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Visibility::Hidden,
    ));

    commands.insert_resource(BunkerSpawned);
}
