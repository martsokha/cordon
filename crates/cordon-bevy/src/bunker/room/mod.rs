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

mod armory;
mod command;
mod entry;
pub mod geometry;
mod quarters;
mod utility;

use avian3d::prelude::*;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::{BunkerSpawned, FpsCamera, InteractPrompt};
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

pub(crate) struct Palette {
    pub concrete: Handle<StandardMaterial>,
    pub concrete_dark: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
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
        }
    }
}

/// Layout constants shared by all zone modules.
pub(crate) struct Layout {
    pub h: f32,
    pub hh: f32,
    pub hw: f32,
    pub front_z: f32,
    pub trade_z: f32,
    pub divider_z: f32,
    pub desk_z: f32,
    pub hole_half: f32,
    pub tj_north: f32,
    pub back_z: f32,
    pub side_depth: f32,
    pub side_door_width: f32,
    pub tj_center: f32,
    pub tj_len: f32,
    pub util_x_min: f32,
    pub util_x_center: f32,
    pub quarters_x_max: f32,
    pub quarters_x_center: f32,
}

impl Layout {
    fn new() -> Self {
        let h = 2.4;
        let hw = 2.0;
        let front_z = 5.0;
        let trade_z = 1.5;
        let tj_north = -3.0;
        let back_z = -6.0;
        let side_depth = 3.0;
        let tj_center = (tj_north + back_z) / 2.0;
        Self {
            h,
            hh: h / 2.0,
            hw,
            front_z,
            trade_z,
            divider_z: -1.5,
            desk_z: trade_z - 0.5,
            hole_half: 0.6,
            tj_north,
            back_z,
            side_depth,
            side_door_width: 1.2,
            tj_center,
            tj_len: tj_north - back_z,
            util_x_min: -(hw + side_depth),
            util_x_center: (-(hw + side_depth) + (-hw)) / 2.0,
            quarters_x_max: hw + side_depth,
            quarters_x_center: (hw + hw + side_depth) / 2.0,
        }
    }
}

fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    use geometry::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    let pal = Palette::new(&mut mats);
    let l = Layout::new();

    // ── Camera ──────────────────────────────────────────────
    let fps_camera_entity = commands
        .spawn((
            FpsCamera,
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(0.0, 1.6, l.desk_z - 0.5)
                .looking_at(Vec3::new(0.0, 1.2, l.front_z), Vec3::Y),
        ))
        .id();

    // ── Lighting ────────────────────────────────────────────
    for (z, intensity, range, shadows) in [
        (l.desk_z, 140000.0, 14.0, true),
        (3.0, 60000.0, 12.0, false),
        (-2.0, 70000.0, 12.0, false),
    ] {
        commands.spawn((
            PointLight {
                intensity, range, shadows_enabled: shadows,
                color: Color::srgb(1.0, 0.85, 0.55), ..default()
            },
            Transform::from_xyz(0.0, l.h - 0.15, z),
        ));
        glb(&mut commands, &asset_server, "models/interior/CeilingLamp.glb",
            Vec3::new(0.0, l.h, z), Quat::IDENTITY);
    }
    commands.spawn((
        PointLight { intensity: 25000.0, color: Color::srgb(1.0, 0.72, 0.40), range: 4.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(l.quarters_x_center, 1.4, l.tj_center - 0.5),
    ));
    glb(&mut commands, &asset_server, "models/interior/StandingLamp.glb",
        Vec3::new(l.quarters_x_center, 0.0, l.tj_center - 0.5), Quat::IDENTITY);
    commands.spawn((
        PointLight { intensity: 8000.0, color: Color::srgb(0.5, 0.8, 0.5), range: 2.5, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 1.1, l.desk_z),
    ));
    commands.spawn((
        PointLight { intensity: 50000.0, color: Color::srgb(1.0, 0.9, 0.7), range: 8.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(l.util_x_center, l.h - 0.15, l.tj_center),
    ));
    glb(&mut commands, &asset_server, "models/interior/CeilingLamp.glb",
        Vec3::new(l.util_x_center, l.h, l.tj_center), Quat::IDENTITY);
    commands.spawn((
        PointLight { intensity: 20000.0, color: Color::srgb(1.0, 0.75, 0.50), range: 5.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(l.quarters_x_center, l.h - 0.15, l.tj_center),
    ));
    glb(&mut commands, &asset_server, "models/interior/Lamp1.glb",
        Vec3::new(0.4, 0.95, l.desk_z), Quat::IDENTITY);

    // ── Main corridor: floor + ceiling ──────────────────────
    let main_center_z = (l.front_z + l.back_z) / 2.0;
    let main_floor_half = Vec2::new(l.hw, (l.front_z - l.back_z) / 2.0);
    spawn_floor_ceiling(&mut commands, &mut meshes, pal.concrete_dark.clone(),
        Vec3::new(0.0, 0.0, main_center_z), main_floor_half, l.h);

    // ── Main corridor: walls ────────────────────────────────
    spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
        Vec3::new(0.0, l.hh, l.front_z), Quat::from_rotation_y(PI), Vec2::new(l.hw, l.hh));

    // Left wall + kitchen doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(-l.hw, l.hh, cz), Quat::from_rotation_y(FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
    }
    spawn_doorframe_x(&mut commands, &mut meshes, pal.concrete.clone(), -l.hw, l.tj_center, l.side_door_width);
    {
        let door_n = l.tj_center + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh, cz), Quat::from_rotation_y(FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
        }
    }
    {
        let door_s = l.tj_center - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh, cz), Quat::from_rotation_y(FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
        }
    }

    // Right wall + bedroom doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
            Vec3::new(l.hw, l.hh, cz), Quat::from_rotation_y(-FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
    }
    spawn_doorframe_x(&mut commands, &mut meshes, pal.concrete.clone(), l.hw, l.tj_center, l.side_door_width);
    {
        let door_n = l.tj_center + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(l.hw, l.hh, cz), Quat::from_rotation_y(-FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
        }
    }
    {
        let door_s = l.tj_center - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
                Vec3::new(l.hw, l.hh, cz), Quat::from_rotation_y(-FRAC_PI_2), Vec2::new(len / 2.0, l.hh));
        }
    }

    // Back wall.
    spawn_wall(&mut commands, &mut meshes, pal.concrete.clone(),
        Vec3::new(0.0, l.hh, l.back_z), Quat::IDENTITY, Vec2::new(l.hw, l.hh));

    // Electric box on the corridor wall near the divider grate.
    glb(&mut commands, &asset_server, "models/storage/ElectricBox_02.glb",
        Vec3::new(l.hw - 0.05, 1.6, l.divider_z + 0.3), Quat::from_rotation_y(-FRAC_PI_2));
    // Filing cabinet against the left corridor wall.
    glb(&mut commands, &asset_server, "models/storage/Cabinet_02.glb",
        Vec3::new(-l.hw + 0.3, 0.0, l.divider_z + 0.5), Quat::from_rotation_y(FRAC_PI_2));

    // ── Zones ───────────────────────────────────────────────
    entry::spawn(&mut commands, &asset_server, &mut meshes, &mut mats, &pal, &l);
    command::spawn(&mut commands, &asset_server, &mut meshes, &mut mats, &pal, &l);
    armory::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);
    utility::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);
    quarters::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);

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
