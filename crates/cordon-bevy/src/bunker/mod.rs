//! 3D bunker scene with FPS camera.

mod input;

use bevy::prelude::*;

use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::InputPlugin);
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            (
                setup_bunker.run_if(not(resource_exists::<BunkerSpawned>)),
                enable_bunker_camera,
            ),
        );
        app.add_systems(OnExit(PlayingState::Bunker), disable_bunker_camera);
    }
}

#[derive(Resource)]
struct BunkerSpawned;

#[derive(Component)]
pub struct FpsCamera;

#[derive(Component)]
pub struct LaptopObject;

#[derive(Component)]
pub struct BunkerUi;

#[derive(Component)]
pub struct InteractPrompt;

fn setup_bunker(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        FpsCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.6, 3.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 2000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.3, 0.0)),
    ));

    commands.spawn((
        PointLight {
            intensity: 50000.0,
            range: 20.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 2.5, 0.0),
    ));

    let floor_material = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.13, 0.12),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let wall_material = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.18, 0.16),
        perceptual_roughness: 0.95,
        ..default()
    });
    for (pos, rot) in [
        (Vec3::new(0.0, 1.5, -5.0), Quat::IDENTITY),
        (
            Vec3::new(-5.0, 1.5, 0.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        ),
        (
            Vec3::new(5.0, 1.5, 0.0),
            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        ),
        (
            Vec3::new(0.0, 1.5, 5.0),
            Quat::from_rotation_y(std::f32::consts::PI),
        ),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(5.0, 1.5)))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos).with_rotation(rot),
        ));
    }

    let table_material = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.18, 0.12),
        perceptual_roughness: 0.8,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.2, 0.05, 0.6))),
        MeshMaterial3d(table_material),
        Transform::from_xyz(0.0, 0.75, 0.0),
    ));

    let laptop_screen = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.15, 0.1),
        emissive: LinearRgba::new(0.1, 0.2, 0.1, 1.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        LaptopObject,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.25, 0.18)))),
        MeshMaterial3d(laptop_screen),
        Transform::from_xyz(0.0, 0.95, -0.05).with_rotation(Quat::from_rotation_x(-0.3)),
    ));

    commands.spawn((
        BunkerUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
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

fn enable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = true;
    }
}

fn disable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = false;
    }
}
