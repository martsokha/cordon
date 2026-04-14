use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::cctv::MonitorPlacement;
use super::components::*;
use super::resources::*;
use super::{geometry, rooms};

pub(super) fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut effects: ResMut<Assets<bevy_hanabi::EffectAsset>>,
) {
    let pal = Palette::new(&mut mats);
    let l = Layout::new();

    commands.insert_resource(MonitorPlacement {
        pos: Vec3::new(-l.hw + 0.15, l.h - 0.25, l.trade_z - 0.1),
        target: Vec3::new(0.0, 1.4, 0.0),
    });

    const TABLE_TOP: f32 = 1.037;
    commands.insert_resource(LaptopPlacement {
        pos: Vec3::new(0.0, TABLE_TOP, l.desk_z()),
        rot: Quat::from_rotation_y(PI),
    });

    let fps_camera_entity = spawn_camera(&mut commands, &l);
    super::lighting::spawn_lighting(&mut commands, &asset_server, &l);
    spawn_corridor(&mut commands, &mut meshes, &pal, &l);

    {
        let mut ctx = RoomCtx {
            commands: &mut commands,
            asset_server: &asset_server,
            meshes: &mut meshes,
            mats: &mut mats,
            effects: &mut effects,
            pal: &pal,
            l: &l,
        };
        rooms::entry::spawn(&mut ctx);
        rooms::command::spawn(&mut ctx);
        rooms::armory::spawn(&mut ctx);
        rooms::kitchen::spawn(&mut ctx);
        rooms::quarters::spawn(&mut ctx);
        rooms::infirmary::spawn(&mut ctx);
        rooms::workshop::spawn(&mut ctx);
    }

    rooms::antechamber::spawn(&mut commands, &mut meshes, &mut mats, &asset_server);

    spawn_ui(&mut commands, fps_camera_entity);
    commands.insert_resource(BunkerSpawned);
}

fn spawn_camera(commands: &mut Commands, l: &Layout) -> Entity {
    commands
        .spawn((
            FpsCamera,
            super::input::controller::StepTracker::default(),
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(
                0.0,
                super::input::controller::CAMERA_EYE_Y,
                l.desk_z() - 0.5,
            )
            .looking_at(Vec3::new(0.0, 1.2, l.front_z), Vec3::Y),
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            // Subtle bloom on emissive surfaces.
            bevy::post_process::bloom::Bloom {
                intensity: 0.08,
                ..default()
            },
            // Fog -- dark haze for depth.
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

fn spawn_corridor(commands: &mut Commands, meshes: &mut Assets<Mesh>, pal: &Palette, l: &Layout) {
    use std::f32::consts::{FRAC_PI_2, PI};

    use geometry::*;

    // Floor + ceiling span the full extended corridor.
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

    // Side walls: each side runs from front_z to back_z with two
    // door openings (one per T-junction) cut into it. The helper
    // emits wall segments for the gaps between door openings.
    spawn_side_wall(
        commands,
        meshes,
        pal,
        l,
        -l.hw,
        Quat::from_rotation_y(FRAC_PI_2),
    );
    spawn_side_wall(
        commands,
        meshes,
        pal,
        l,
        l.hw,
        Quat::from_rotation_y(-FRAC_PI_2),
    );

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

/// Spawn one side of the main corridor: a wall that runs
/// `front_z → back_z` with T1 and T2 door openings cut into it.
/// Emits up to five wall segments (front stub, between T1 door and
/// T2 door, back stub, plus the narrow strips above/below each
/// door) and a doorframe at each T's centre.
///
/// `x` is the wall's x-coordinate (`-hw` for left, `+hw` for
/// right); `rot` orients the wall facing the corridor interior.
fn spawn_side_wall(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    l: &Layout,
    x: f32,
    rot: Quat,
) {
    use geometry::*;

    // Four "gap" spans along the Z axis where the wall is solid
    // (not interrupted by a T door). Each entry is (z_north, z_south).
    let gaps = [
        // Front stub: front_z → T1 door north edge.
        (l.front_z, l.tj1_center() + l.side_door_width / 2.0),
        // Between T1 door south edge and T2 door north edge.
        (
            l.tj1_center() - l.side_door_width / 2.0,
            l.tj2_center() + l.side_door_width / 2.0,
        ),
        // Back stub: T2 door south edge → back_z.
        (l.tj2_center() - l.side_door_width / 2.0, l.back_z),
    ];
    for (n, s) in gaps {
        let len = (n - s).abs();
        if len <= 0.1 {
            continue;
        }
        let cz = (n + s) / 2.0;
        spawn_wall(
            commands,
            meshes,
            pal.concrete.clone(),
            Vec3::new(x, l.hh(), cz),
            rot,
            Vec2::new(len / 2.0, l.hh()),
        );
    }

    // Doorframes at each T-junction centre.
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        x,
        l.tj1_center(),
        l.side_door_width,
        l.opening_h(),
    );
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        x,
        l.tj2_center(),
        l.side_door_width,
        l.opening_h(),
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
