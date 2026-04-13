use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::cctv::components::MonitorPlacement;
use super::components::*;
use super::geometry;
use super::resources::*;
use super::rooms;

pub(super) fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let pal = Palette::new(&mut mats);
    let l = Layout::new();

    commands.insert_resource(MonitorPlacement {
        pos: Vec3::new(-l.hw + 0.15, l.h - 0.25, l.trade_z - 0.1),
        target: Vec3::new(0.0, 1.4, 0.0),
    });

    let fps_camera_entity = spawn_camera(&mut commands, &l);
    super::lighting::spawn_lighting(&mut commands, &asset_server, &l);
    spawn_corridor(&mut commands, &mut meshes, &pal, &l);

    let mut ctx = RoomCtx {
        commands: &mut commands,
        asset_server: &asset_server,
        meshes: &mut meshes,
        mats: &mut mats,
        pal: &pal,
        l: &l,
    };
    rooms::entry::spawn(&mut ctx);
    rooms::command::spawn(&mut ctx);
    rooms::armory::spawn(&mut ctx);
    rooms::kitchen::spawn(&mut ctx);
    rooms::quarters::spawn(&mut ctx);
    drop(ctx);

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
        l.opening_h(),
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
        l.opening_h(),
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
