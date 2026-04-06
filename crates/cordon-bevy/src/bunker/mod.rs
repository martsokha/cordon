//! 3D bunker scene with FPS camera.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::PlayingState;

pub struct BunkerPlugin;

impl Plugin for BunkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            (setup_bunker.run_if(not(resource_exists::<BunkerSpawned>)), grab_cursor, enable_bunker_camera),
        );
        app.add_systems(OnEnter(PlayingState::Laptop), hide_interact_prompt);
        app.add_systems(OnExit(PlayingState::Bunker), (release_cursor, disable_bunker_camera));
        app.add_systems(
            Update,
            (fps_look, fps_move, update_interact_prompt, interact)
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}

#[derive(Resource)]
struct BunkerSpawned;

#[derive(Component)]
struct FpsCamera;

#[derive(Component)]
struct LaptopObject;

#[derive(Component)]
struct BunkerUi;

#[derive(Component)]
struct InteractPrompt;

const MOVE_SPEED: f32 = 4.0;
const LOOK_SENSITIVITY: f32 = 0.003;

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
        (Vec3::new(-5.0, 1.5, 0.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(5.0, 1.5, 0.0), Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        (Vec3::new(0.0, 1.5, 5.0), Quat::from_rotation_y(std::f32::consts::PI)),
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
        Transform::from_xyz(0.0, 0.95, -0.05)
            .with_rotation(Quat::from_rotation_x(-0.3)),
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
        TextFont { font_size: 20.0, ..default() },
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
        TextFont { font_size: 14.0, ..default() },
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

fn grab_cursor(mut cursor_q: Query<&mut CursorOptions>) {
    for mut cursor in &mut cursor_q {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn release_cursor(mut cursor_q: Query<&mut CursorOptions>) {
    for mut cursor in &mut cursor_q {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn fps_look(
    mut motion: MessageReader<MouseMotion>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let delta: Vec2 = motion.read().map(|e| e.delta).sum();
    if delta == Vec2::ZERO {
        return;
    }

    for mut transform in &mut camera_q {
        let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        yaw -= delta.x * LOOK_SENSITIVITY;
        pitch -= delta.y * LOOK_SENSITIVITY;
        pitch = pitch.clamp(-1.4, 1.4);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

fn fps_move(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
) {
    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { input.y += 1.0; }
    if keys.pressed(KeyCode::KeyS) { input.y -= 1.0; }
    if keys.pressed(KeyCode::KeyA) { input.x -= 1.0; }
    if keys.pressed(KeyCode::KeyD) { input.x += 1.0; }
    if input == Vec2::ZERO {
        return;
    }

    for mut transform in &mut camera_q {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let movement = (flat_forward * input.y + flat_right * input.x).normalize_or_zero()
            * MOVE_SPEED
            * time.delta_secs();
        transform.translation += movement;
        transform.translation.x = transform.translation.x.clamp(-4.5, 4.5);
        transform.translation.z = transform.translation.z.clamp(-4.5, 4.5);
    }
}

fn hide_interact_prompt(mut prompt_q: Query<&mut Visibility, With<InteractPrompt>>) {
    for mut vis in &mut prompt_q {
        *vis = Visibility::Hidden;
    }
}

fn can_interact(
    camera_q: &Query<&Transform, With<FpsCamera>>,
    laptop_q: &Query<&Transform, With<LaptopObject>>,
) -> bool {
    let Ok(cam) = camera_q.single() else { return false };
    let Ok(laptop) = laptop_q.single() else { return false };
    cam.translation.distance(laptop.translation) < 2.0
}

fn update_interact_prompt(
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    mut prompt_q: Query<(&mut Text, &mut Visibility), With<InteractPrompt>>,
) {
    let near = can_interact(&camera_q, &laptop_q);
    for (mut text, mut vis) in &mut prompt_q {
        if near {
            text.0 = "[E] Use Laptop".into();
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    laptop_q: Query<&Transform, With<LaptopObject>>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Ok(cam) = camera_q.single() else { return };
    let Ok(laptop) = laptop_q.single() else { return };

    let dist = cam.translation.distance(laptop.translation);
    if dist < 2.0 {
        *next_state = NextState::Pending(PlayingState::Laptop);
    }
}
