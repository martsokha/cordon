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
mod kitchen;
mod quarters;

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
            color: Color::srgb(0.9, 0.85, 0.70),
            brightness: 80.0,
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

/// Bunker dimensions. Only stores the primary constants; derived
/// values are computed via methods so nothing can go stale.
pub(crate) struct Layout {
    /// Ceiling height.
    pub h: f32,
    /// Main corridor half-width (x extent from center).
    pub hw: f32,
    /// Z of the front wall (stairs / entrance).
    pub front_z: f32,
    /// Z of the trade grate.
    pub trade_z: f32,
    /// Z of the office ↔ armory divider grate.
    pub divider_z: f32,
    /// Half-width of the grate opening.
    pub hole_half: f32,
    /// Z of the north edge of the T-junction.
    pub tj_north: f32,
    /// Z of the back wall (south edge of corridor + side rooms).
    pub back_z: f32,
    /// How far each side room extends from the corridor wall.
    pub side_depth: f32,
    /// Width of the side-room doorframe openings.
    pub side_door_width: f32,
}

impl Layout {
    fn new() -> Self {
        Self {
            h: 2.4,
            hw: 2.0,
            front_z: 5.0,
            trade_z: 1.5,
            divider_z: -1.5,
            hole_half: 0.6,
            tj_north: -3.0,
            back_z: -6.0,
            side_depth: 3.0,
            side_door_width: 1.6,
        }
    }

    pub fn hh(&self) -> f32 {
        self.h / 2.0
    }

    pub fn desk_z(&self) -> f32 {
        self.trade_z - 0.5
    }

    pub fn tj_center(&self) -> f32 {
        (self.tj_north + self.back_z) / 2.0
    }

    pub fn tj_len(&self) -> f32 {
        self.tj_north - self.back_z
    }

    /// Kitchen (left): furthest x.
    pub fn kitchen_x_min(&self) -> f32 {
        -(self.hw + self.side_depth)
    }

    /// Kitchen (left): center x.
    pub fn kitchen_x_center(&self) -> f32 {
        (self.kitchen_x_min() + (-self.hw)) / 2.0
    }

    /// Quarters (right): furthest x.
    pub fn quarters_x_max(&self) -> f32 {
        self.hw + self.side_depth
    }

    /// Quarters (right): center x.
    pub fn quarters_x_center(&self) -> f32 {
        (self.hw + self.quarters_x_max()) / 2.0
    }
}

fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let pal = Palette::new(&mut mats);
    let l = Layout::new();

    let fps_camera_entity = spawn_camera(&mut commands, &l);
    spawn_lighting(&mut commands, &asset_server, &l);
    spawn_corridor(&mut commands, &mut meshes, &pal, &l);

    entry::spawn(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut mats,
        &pal,
        &l,
    );
    command::spawn(
        &mut commands,
        &asset_server,
        &mut meshes,
        &mut mats,
        &pal,
        &l,
    );
    armory::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);
    kitchen::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);
    quarters::spawn(&mut commands, &asset_server, &mut meshes, &pal, &l);

    spawn_ui(&mut commands, fps_camera_entity);
    commands.insert_resource(BunkerSpawned);
}

fn spawn_camera(commands: &mut Commands, l: &Layout) -> Entity {
    commands
        .spawn((
            FpsCamera,
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(0.0, 1.6, l.desk_z() - 0.5)
                .looking_at(Vec3::new(0.0, 1.2, l.front_z), Vec3::Y),
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            // Subtle bloom on emissive surfaces.
            bevy::post_process::bloom::Bloom {
                intensity: 0.08,
                ..default()
            },
            // Fog — dark haze for depth.
            bevy::pbr::DistanceFog {
                color: Color::srgba(0.04, 0.04, 0.05, 1.0),
                falloff: bevy::pbr::FogFalloff::Linear {
                    start: 8.0,
                    end: 16.0,
                },
                ..default()
            },
        ))
        .id()
}

fn spawn_lighting(commands: &mut Commands, asset_server: &AssetServer, l: &Layout) {
    use geometry::LightFixture;

    let warm = Color::srgb(1.0, 0.82, 0.50);
    let cool = Color::srgb(0.85, 0.9, 1.0);
    let dim_cool = Color::srgb(0.8, 0.85, 0.95);
    let white = Color::srgb(0.95, 0.95, 1.0);
    let dim_warm = Color::srgb(1.0, 0.75, 0.45);
    let lamp_warm = Color::srgb(1.0, 0.70, 0.35);
    let screen_green = Color::srgb(0.4, 0.7, 0.4);

    let fixtures = [
        // Command post — ceiling lamp pulled 1m back from the desk
        // so it illuminates the room, not just the table surface.
        LightFixture::ceiling(0.0, l.desk_z() - 1.0, l.h, 120000.0, warm, true),
        LightFixture::desk(Vec3::new(0.4, 0.95, l.desk_z() - 0.15), 8000.0, warm),
        LightFixture::screen(Vec3::new(0.0, 1.1, l.desk_z()), 6000.0, screen_green),
        // Entry
        LightFixture::ceiling(0.0, 3.0, l.h, 50000.0, cool, false),
        // Armory + T-junction — single light between them.
        LightFixture::ceiling(0.0, l.tj_north - 0.5, l.h, 50000.0, dim_cool, false),
        // Kitchen
        LightFixture::ceiling(
            l.kitchen_x_center(),
            l.tj_center(),
            l.h,
            45000.0,
            white,
            false,
        ),
        // Quarters
        LightFixture::ceiling(
            l.quarters_x_center(),
            l.tj_center(),
            l.h,
            15000.0,
            dim_warm,
            false,
        ),
        LightFixture::standing(
            l.quarters_x_center(),
            l.tj_center() - 0.5,
            18000.0,
            lamp_warm,
        ),
    ];

    for fixture in &fixtures {
        fixture.spawn(commands, asset_server);
    }
}

fn spawn_corridor(commands: &mut Commands, meshes: &mut Assets<Mesh>, pal: &Palette, l: &Layout) {
    use std::f32::consts::{FRAC_PI_2, PI};

    use geometry::*;

    // Floor + ceiling.
    let main_center_z = (l.front_z + l.back_z) / 2.0;
    let main_floor_half = Vec2::new(l.hw, (l.front_z - l.back_z) / 2.0);
    spawn_floor_ceiling(
        commands,
        meshes,
        pal.concrete_dark.clone(),
        Vec3::new(0.0, 0.0, main_center_z),
        main_floor_half,
        l.h,
    );

    // Front wall.
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(0.0, l.hh(), l.front_z),
        Quat::from_rotation_y(PI),
        Vec2::new(l.hw, l.hh()),
    );

    // Left wall + kitchen doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(
            commands,
            meshes,
            pal.concrete.clone(),
            Vec3::new(-l.hw, l.hh(), cz),
            Quat::from_rotation_y(FRAC_PI_2),
            Vec2::new(len / 2.0, l.hh()),
        );
    }
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        -l.hw,
        l.tj_center(),
        l.side_door_width,
    );
    {
        let door_n = l.tj_center() + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh(), cz),
                Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }
    {
        let door_s = l.tj_center() - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh(), cz),
                Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }

    // Right wall + bedroom doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(
            commands,
            meshes,
            pal.concrete.clone(),
            Vec3::new(l.hw, l.hh(), cz),
            Quat::from_rotation_y(-FRAC_PI_2),
            Vec2::new(len / 2.0, l.hh()),
        );
    }
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        l.hw,
        l.tj_center(),
        l.side_door_width,
    );
    {
        let door_n = l.tj_center() + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(l.hw, l.hh(), cz),
                Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }
    {
        let door_s = l.tj_center() - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(l.hw, l.hh(), cz),
                Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }

    // Back wall.
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(0.0, l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.hw, l.hh()),
    );
}

fn spawn_ui(commands: &mut Commands, fps_camera_entity: Entity) {
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
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Visibility::Hidden,
    ));
}
