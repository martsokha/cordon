//! 3D bunker scene with FPS camera.

mod input;

use bevy::prelude::*;

use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(input::InputPlugin);
        app.insert_resource(CameraMode::Free);
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            (
                setup_bunker.run_if(not(resource_exists::<BunkerSpawned>)),
                enable_bunker_camera,
            ),
        );
        app.add_systems(OnExit(PlayingState::Bunker), disable_bunker_camera);
        app.add_systems(OnEnter(PlayingState::Laptop), start_laptop_zoom);
        app.add_systems(OnEnter(PlayingState::Bunker), start_free_look);
        app.add_systems(Update, animate_camera);
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
    // Laptop screen: vertical plane facing +Z, tilted back ~20 degrees
    commands.spawn((
        LaptopObject,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.35, 0.25)))),
        MeshMaterial3d(laptop_screen),
        Transform::from_xyz(0.0, 0.92, -0.1).with_rotation(Quat::from_rotation_x(-0.35)),
    ));

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

/// Camera position when zoomed into the laptop screen.
/// Computed along the screen's normal from its center.
const LAPTOP_VIEW_POS: Vec3 = Vec3::new(0.0, 1.156, 0.464);
const LAPTOP_VIEW_TARGET: Vec3 = Vec3::new(0.0, 0.96, -0.1);
const CAMERA_LERP_SPEED: f32 = 5.0;

#[derive(Resource, Clone)]
enum CameraMode {
    Free,
    ZoomingToLaptop { saved_transform: Transform },
    AtLaptop { saved_transform: Transform },
    Returning,
}

fn start_laptop_zoom(camera_q: Query<&Transform, With<FpsCamera>>, mut mode: ResMut<CameraMode>) {
    if let Ok(transform) = camera_q.single() {
        *mode = CameraMode::ZoomingToLaptop {
            saved_transform: *transform,
        };
    }
}

fn start_free_look(mut mode: ResMut<CameraMode>) {
    match &*mode {
        CameraMode::AtLaptop { .. } | CameraMode::ZoomingToLaptop { .. } => {
            *mode = CameraMode::Returning;
        }
        _ => {}
    }
}

fn animate_camera(
    time: Res<Time>,
    mut mode: ResMut<CameraMode>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let dt = time.delta_secs();
    let factor = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();

    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    match mode.clone() {
        CameraMode::Free | CameraMode::Returning => {
            if matches!(*mode, CameraMode::Returning) {
                // Returning is handled by the FPS controller taking over
                *mode = CameraMode::Free;
            }
        }
        CameraMode::ZoomingToLaptop { saved_transform } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;

            transform.translation = transform.translation.lerp(LAPTOP_VIEW_POS, factor);
            transform.rotation = transform.rotation.slerp(target_rot, factor);

            if transform.translation.distance(LAPTOP_VIEW_POS) < 0.01 {
                *mode = CameraMode::AtLaptop { saved_transform };
            }
        }
        CameraMode::AtLaptop { .. } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;
            transform.translation = LAPTOP_VIEW_POS;
            transform.rotation = target_rot;
        }
    }
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
