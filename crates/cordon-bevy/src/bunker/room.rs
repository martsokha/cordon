//! Bunker room geometry, lighting, and physics colliders.
//!
//! T-shaped underground bunker: a main north-south corridor with
//! two side rooms branching off the armory at the south end.
//!
//! ```text
//!     z = 5.0  STAIRS DOWN FROM SURFACE
//!              ENTRY CHECKPOINT
//!     z = 1.5  TRADE GRATE
//!              COMMAND POST
//!     z =-1.5  DIVIDER GRATE
//!              ARMORY / SUPPLY CACHE
//!     z =-3.0  ─────── T-JUNCTION ───────
//!              │                         │
//!    UTILITY (left)              QUARTERS (right)
//!    x: -2 to -5                 x: 2 to 5
//!    z: -3.0 to -6.0            z: -3.0 to -6.0
//! ```

use avian3d::prelude::*;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::{BunkerSpawned, DoorButton, FpsCamera, InteractPrompt, LaptopObject};
use crate::PlayingState;

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlobalAmbientLight {
            color: Color::srgb(1.0, 0.85, 0.65),
            brightness: 120.0,
            ..default()
        });
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            spawn_bunker.run_if(not(resource_exists::<BunkerSpawned>)),
        );
    }
}

struct Palette {
    concrete: Handle<StandardMaterial>,
    concrete_dark: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    metal_dark: Handle<StandardMaterial>,
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
        }
    }
}

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

/// Spawn a wall as a thin cuboid. Visible from both sides and
/// acts as its own physics collider — no separate collider entity
/// needed. `half_size.x` = half-width, `half_size.y` = half-height.
/// The wall is 0.08m thick.
fn spawn_wall(
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

fn spawn_floor_ceiling(
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

fn spawn_doorframe(
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

fn spawn_doorframe_x(
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

fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    use std::f32::consts::{FRAC_PI_2, PI};

    let pal = Palette::new(&mut mats);

    let h = 2.4;
    let hh = h / 2.0;
    let hw = 2.0;

    let front_z = 5.0;
    let trade_z = 1.5;
    let divider_z = -1.5;
    let desk_z = trade_z - 0.5;
    let hole_half = 0.6;

    // T-junction geometry.
    let tj_north = -3.0;
    let back_z = -6.0;
    let side_depth = 3.0;
    let side_door_width = 1.2;
    let tj_center = (tj_north + back_z) / 2.0;
    let tj_len = tj_north - back_z;

    let util_x_min = -(hw + side_depth);
    let util_x_center = (util_x_min + (-hw)) / 2.0;
    let quarters_x_max = hw + side_depth;
    let quarters_x_center = (hw + quarters_x_max) / 2.0;

    let glb =
        |commands: &mut Commands, asset_server: &AssetServer, path: &str, pos: Vec3, rot: Quat| {
            let scene: Handle<Scene> = asset_server.load(format!("{path}#Scene0"));
            commands.spawn((
                SceneRoot(scene),
                Transform::from_translation(pos).with_rotation(rot),
            ));
        };

    // ── Camera ──────────────────────────────────────────────
    let fps_camera_entity = commands
        .spawn((
            FpsCamera,
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(0.0, 1.6, desk_z - 0.5)
                .looking_at(Vec3::new(0.0, 1.2, front_z), Vec3::Y),
        ))
        .id();

    // ── Lighting ────────────────────────────────────────────
    // Industrial ceiling lights down the main corridor.
    for (z, intensity, range, shadows) in [
        (desk_z, 140000.0, 14.0, true),
        (3.0, 60000.0, 12.0, false),
        (-2.0, 70000.0, 12.0, false),
    ] {
        commands.spawn((
            PointLight {
                intensity, range, shadows_enabled: shadows,
                color: Color::srgb(1.0, 0.85, 0.55),
                ..default()
            },
            Transform::from_xyz(0.0, h - 0.15, z),
        ));
        glb(&mut commands, &asset_server, "models/interior/CeilingLamp.glb",
            Vec3::new(0.0, h, z), Quat::IDENTITY);
    }
    // Standing lamp in quarters.
    commands.spawn((
        PointLight {
            intensity: 25000.0, color: Color::srgb(1.0, 0.72, 0.40),
            range: 4.0, shadows_enabled: false, ..default()
        },
        Transform::from_xyz(quarters_x_center, 1.4, tj_center - 0.5),
    ));
    glb(&mut commands, &asset_server, "models/interior/StandingLamp.glb",
        Vec3::new(quarters_x_center, 0.0, tj_center - 0.5), Quat::IDENTITY);
    // Monitor glow.
    commands.spawn((
        PointLight {
            intensity: 8000.0, color: Color::srgb(0.5, 0.8, 0.5),
            range: 2.5, shadows_enabled: false, ..default()
        },
        Transform::from_xyz(0.0, 1.1, desk_z),
    ));
    // Utility room light.
    commands.spawn((
        PointLight {
            intensity: 50000.0, color: Color::srgb(1.0, 0.9, 0.7),
            range: 8.0, shadows_enabled: false, ..default()
        },
        Transform::from_xyz(util_x_center, h - 0.15, tj_center),
    ));
    glb(&mut commands, &asset_server, "models/interior/CeilingLamp.glb",
        Vec3::new(util_x_center, h, tj_center), Quat::IDENTITY);
    // Quarters light — bedside lamp only, dimmer.
    commands.spawn((
        PointLight {
            intensity: 20000.0, color: Color::srgb(1.0, 0.75, 0.50),
            range: 5.0, shadows_enabled: false, ..default()
        },
        Transform::from_xyz(quarters_x_center, h - 0.15, tj_center),
    ));
    // Small lamp on the command desk.
    glb(&mut commands, &asset_server, "models/interior/Lamp1.glb",
        Vec3::new(0.4, 0.77, desk_z), Quat::IDENTITY);

    // ── Main corridor: floor + ceiling ──────────────────────
    let main_center_z = (front_z + back_z) / 2.0;
    let main_floor_half = Vec2::new(hw, (front_z - back_z) / 2.0);
    spawn_floor_ceiling(&mut commands, &mut meshes, pal.concrete_dark.clone(),
        Vec3::new(0.0, 0.0, main_center_z), main_floor_half, h);

    // ── Main corridor: walls ────────────────────────────────
    // Front wall.
    spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
        Vec3::new(0.0, hh, front_z), Quat::from_rotation_y(PI), Vec2::new(hw, hh));

    // Left wall segments + kitchen doorframe.
    // Upper segment: front_z to tj_north.
    {
        let len = front_z - tj_north;
        let cz = (front_z + tj_north) / 2.0;
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(-hw, hh, cz), Quat::from_rotation_y(FRAC_PI_2),
            Vec2::new(len / 2.0, hh));
    }
    // Doorframe.
    spawn_doorframe_x(&mut commands, &mut meshes, pal.concrete.clone(),
        -hw, tj_center, side_door_width);
    // North stub: tj_north to door-north-edge.
    {
        let door_n = tj_center + side_door_width / 2.0;
        let len = (tj_north - door_n).abs();
        let cz = (tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(-hw, hh, cz), Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, hh));
        }
    }
    // South stub: door-south-edge to back_z.
    {
        let door_s = tj_center - side_door_width / 2.0; // -5.1
        let len = (door_s - back_z).abs();
        let cz = (door_s + back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(-hw, hh, cz), Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, hh));
        }
    }

    // Right wall segments + bedroom doorframe (mirror of left).
    {
        let len = front_z - tj_north;
        let cz = (front_z + tj_north) / 2.0;
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(hw, hh, cz), Quat::from_rotation_y(-FRAC_PI_2),
            Vec2::new(len / 2.0, hh));
    }
    spawn_doorframe_x(&mut commands, &mut meshes, pal.concrete.clone(),
        hw, tj_center, side_door_width);
    {
        let door_n = tj_center + side_door_width / 2.0;
        let len = (tj_north - door_n).abs();
        let cz = (tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(hw, hh, cz), Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, hh));
        }
    }
    {
        let door_s = tj_center - side_door_width / 2.0;
        let len = (door_s - back_z).abs();
        let cz = (door_s + back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(hw, hh, cz), Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, hh));
        }
    }

    // Back wall (between the side room openings).
    spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
        Vec3::new(0.0, hh, back_z), Quat::IDENTITY, Vec2::new(hw, hh));

    // ── Utility room (left) ─────────────────────────────────
    {
        let floor_half = Vec2::new(side_depth / 2.0, tj_len / 2.0);
        spawn_floor_ceiling(&mut commands, &mut meshes, pal.concrete_dark.clone(),
            Vec3::new(util_x_center, 0.0, tj_center), floor_half, h);
        // Far wall (west).
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(util_x_min, hh, tj_center), Quat::from_rotation_y(-FRAC_PI_2),
            Vec2::new(tj_len / 2.0, hh));
        // North wall.
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(util_x_center, hh, tj_north), Quat::from_rotation_y(PI),
            Vec2::new(side_depth / 2.0, hh));
        // South wall.
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(util_x_center, hh, back_z), Quat::IDENTITY,
            Vec2::new(side_depth / 2.0, hh));

        // Fridge (stores meds and rations).
        glb(&mut commands, &asset_server, "models/interior/AmericanFridge.glb",
            Vec3::new(util_x_min + 0.4, 0.0, tj_north - 0.4), Quat::from_rotation_y(FRAC_PI_2));
        // Kitchen shelves along the far wall.
        glb(&mut commands, &asset_server, "models/interior/KitchenShelves1.glb",
            Vec3::new(util_x_min + 0.3, 0.0, tj_center), Quat::from_rotation_y(FRAC_PI_2));
        // Microwave on the shelves (away from the fridge).
        glb(&mut commands, &asset_server, "models/interior/Microwave.glb",
            Vec3::new(util_x_min + 0.4, 0.9, tj_center - 0.3), Quat::from_rotation_y(FRAC_PI_2));
        // Kettle.
        glb(&mut commands, &asset_server, "models/interior/Kettle.glb",
            Vec3::new(util_x_min + 0.4, 0.9, tj_center + 0.3), Quat::from_rotation_y(FRAC_PI_2));
    }

    // ── Quarters (right) ────────────────────────────────────
    {
        let floor_half = Vec2::new(side_depth / 2.0, tj_len / 2.0);
        spawn_floor_ceiling(&mut commands, &mut meshes, pal.concrete_dark.clone(),
            Vec3::new(quarters_x_center, 0.0, tj_center), floor_half, h);
        // Far wall (east).
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(quarters_x_max, hh, tj_center), Quat::from_rotation_y(FRAC_PI_2),
            Vec2::new(tj_len / 2.0, hh));
        // North wall.
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(quarters_x_center, hh, tj_north), Quat::from_rotation_y(PI),
            Vec2::new(side_depth / 2.0, hh));
        // South wall.
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(quarters_x_center, hh, back_z), Quat::IDENTITY,
            Vec2::new(side_depth / 2.0, hh));

        // Wide sofa against the far wall (doubles as a bed).
        glb(&mut commands, &asset_server, "models/interior/WideSofa.glb",
            Vec3::new(quarters_x_max - 0.5, 0.0, tj_center), Quat::from_rotation_y(-FRAC_PI_2));
        // Pillow on the sofa.
        glb(&mut commands, &asset_server, "models/interior/Pillow.glb",
            Vec3::new(quarters_x_max - 0.5, 0.4, tj_center + 0.5), Quat::IDENTITY);
        // Rug beside the bed (scavenged comfort).
        glb(&mut commands, &asset_server, "models/interior/Rug.glb",
            Vec3::new(quarters_x_center, 0.01, tj_center), Quat::IDENTITY);
    }

    // ── 1. Entry checkpoint (front_z to trade_z) ────────────
    spawn_doorframe(&mut commands, &mut meshes, pal.concrete.clone(), 0.0, front_z - 0.1, 1.0);
    spawn_stairs(&mut commands, &mut meshes, pal.concrete.clone(), front_z + 0.3, 1.0, 6);

    // Trade grate: bars on both sides with a hole in the center
    // at mid-height for passing items through. The counter shelf
    // sits at the bottom of the opening.
    spawn_grate_bars(&mut commands, &mut meshes, pal.metal.clone(),
        -hw, -hole_half, trade_z, h, 0.1);
    spawn_grate_bars(&mut commands, &mut meshes, pal.metal.clone(),
        hole_half, hw, trade_z, h, 0.1);
    // Counter shelf at the opening.
    spawn_box(&mut commands, &mut meshes, pal.wood.clone(),
        Vec3::new(0.0, 0.78, trade_z), Vec3::new(hole_half * 2.0 + 0.2, 0.04, 0.25));
    // Bars below the counter (floor to counter height) to block
    // walking through.
    spawn_grate_bars(&mut commands, &mut meshes, pal.metal.clone(),
        -hole_half, hole_half, trade_z, 0.76, 0.1);

    glb(&mut commands, &asset_server, "models/interior/WoodenStool.glb",
        Vec3::new(0.0, 0.0, trade_z + 0.6), Quat::IDENTITY);
    for i in 0..5 {
        spawn_locker(&mut commands, &mut meshes, pal.metal_dark.clone(),
            Vec3::new(-hw + 0.3, 0.0, 2.2 + 0.5 * i as f32));
    }
    spawn_box(&mut commands, &mut meshes, mats.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.17),
        perceptual_roughness: 0.6, metallic: 0.5, ..default()
    }), Vec3::new(hw - 0.4, 0.3, 3.0), Vec3::new(0.7, 0.6, 1.0));

    // ── 2. Command post (trade_z to divider_z) ──────────────
    spawn_grate_bars(&mut commands, &mut meshes, pal.metal.clone(),
        -hw, -hole_half, divider_z, h, 0.12);
    spawn_grate_bars(&mut commands, &mut meshes, pal.metal.clone(),
        hole_half, hw, divider_z, h, 0.12);

    // Dinner table as the command desk.
    glb(&mut commands, &asset_server, "models/interior/WoodenDinnerTable.glb",
        Vec3::new(0.0, 0.0, desk_z), Quat::IDENTITY);
    // Chair.
    glb(&mut commands, &asset_server, "models/interior/WoodenChair.glb",
        Vec3::new(0.0, 0.0, desk_z - 0.5), Quat::IDENTITY);
    // Laptop (command interface) on the table. The `LaptopObject`
    // marker lets the interaction system detect "looking at the
    // laptop" from the scene root's transform.
    {
        let scene: Handle<Scene> = asset_server.load("models/interior/Laptop.glb#Scene0");
        commands.spawn((
            LaptopObject,
            SceneRoot(scene),
            Transform::from_xyz(0.0, 0.77, desk_z)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ));
    }
    // Mug.
    glb(&mut commands, &asset_server, "models/interior/Mug.glb",
        Vec3::new(-0.35, 0.77, desk_z + 0.05), Quat::IDENTITY);
    // Bin.
    {
        let scene: Handle<Scene> = asset_server.load("models/interior/Bin.glb#Scene0");
        commands.spawn((
            SceneRoot(scene),
            Transform::from_xyz(-0.6, 0.0, desk_z - 0.2).with_scale(Vec3::splat(0.6)),
        ));
    }
    // Full bookshelves (intel files, maps, manuals).
    for z in [-0.6, 0.8] {
        glb(&mut commands, &asset_server, "models/interior/Bookshelf.glb",
            Vec3::new(-hw + 0.3, 0.0, z), Quat::from_rotation_y(FRAC_PI_2));
        glb(&mut commands, &asset_server, "models/interior/Bookshelf.glb",
            Vec3::new(hw - 0.3, 0.0, z), Quat::from_rotation_y(-FRAC_PI_2));
    }
    // Rug in front of the desk.
    glb(&mut commands, &asset_server, "models/interior/Rug.glb",
        Vec3::new(0.0, 0.02, desk_z - 0.3), Quat::IDENTITY);
    // The one living thing down here — cactus in a pot.
    glb(&mut commands, &asset_server, "models/interior/PlantPot1.glb",
        Vec3::new(-hw + 0.3, 0.0, 0.0), Quat::IDENTITY);
    glb(&mut commands, &asset_server, "models/interior/Cactus.glb",
        Vec3::new(-hw + 0.3, 0.25, 0.0), Quat::IDENTITY);
    // Door button.
    commands.spawn((
        DoorButton,
        Mesh3d(meshes.add(Sphere::new(0.025))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.05, 0.05),
            perceptual_roughness: 0.4, metallic: 0.2,
            emissive: LinearRgba::BLACK, ..default()
        })),
        Transform::from_xyz(0.28, 0.8, desk_z),
    ));

    // ── 3. Armory / supply cache (divider_z to tj_north) ────
    glb(&mut commands, &asset_server, "models/interior/Bookshelf.glb",
        Vec3::new(-hw + 0.3, 0.0, -2.2), Quat::from_rotation_y(FRAC_PI_2));
    glb(&mut commands, &asset_server, "models/interior/Bookshelf.glb",
        Vec3::new(hw - 0.3, 0.0, -2.2), Quat::from_rotation_y(-FRAC_PI_2));
    // Armchair at the very back, angled 45° looking toward the desk.
    glb(&mut commands, &asset_server, "models/interior/Armchair1.glb",
        Vec3::new(-hw + 0.5, 0.0, back_z + 0.5), Quat::from_rotation_y(FRAC_PI_2 / 2.0));

    // Back door (boarded up / sealed).
    spawn_box(&mut commands, &mut meshes, pal.wood.clone(),
        Vec3::new(0.0, 1.0, back_z + 0.05), Vec3::new(0.9, 2.0, 0.08));

    // ── UI: interact prompt ─────────────────────────────────
    commands.spawn((
        InteractPrompt,
        UiTargetCamera(fps_camera_entity),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(55.0),
            ..default()
        },
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Visibility::Hidden,
    ));

    commands.insert_resource(BunkerSpawned);
}
