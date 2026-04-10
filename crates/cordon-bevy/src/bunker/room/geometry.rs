//! Low-level geometry spawn helpers: walls, floors, grates, boxes,
//! light fixtures.

use avian3d::prelude::*;
use bevy::prelude::*;

/// A light fixture that spawns both a GLB model and a matching
/// point light at the correct position. The model path determines
/// what kind of fixture it is (ceiling lamp, standing lamp, etc.).
pub struct LightFixture {
    /// Asset path for the fixture model (e.g. "models/interior/CeilingLamp.glb").
    pub model: &'static str,
    /// Where to place the model's origin.
    pub model_pos: Vec3,
    /// Rotation for the model.
    pub model_rot: Quat,
    /// Where the actual light source sits (the bulb, not the base).
    pub light_pos: Vec3,
    /// Light intensity in lumens.
    pub intensity: f32,
    /// Light color.
    pub color: Color,
    /// Light range.
    pub range: f32,
    /// Whether to cast shadows.
    pub shadows: bool,
}

impl LightFixture {
    /// Ceiling lamp with the bulb hanging ~0.35m below the ceiling.
    pub fn ceiling(x: f32, z: f32, ceiling_h: f32, intensity: f32, color: Color, shadows: bool) -> Self {
        Self {
            model: "models/interior/CeilingLamp.glb",
            model_pos: Vec3::new(x, ceiling_h, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, ceiling_h - 0.35, z),
            intensity,
            color,
            range: 10.0,
            shadows,
        }
    }

    /// Standing floor lamp with the bulb at shade height (~1.4m).
    pub fn standing(x: f32, z: f32, intensity: f32, color: Color) -> Self {
        Self {
            model: "models/interior/StandingLamp.glb",
            model_pos: Vec3::new(x, 0.0, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, 1.4, z),
            intensity,
            color,
            range: 3.5,
            shadows: false,
        }
    }

    /// Small desk/table lamp. Model placed at `pos`, light slightly above.
    pub fn desk(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: "models/interior/Lamp1.glb",
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos + Vec3::new(0.0, 0.3, 0.0),
            intensity,
            color,
            range: 2.5,
            shadows: false,
        }
    }

    /// A screen glow (no model — the screen itself is the source).
    pub fn screen(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: "",
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos,
            intensity,
            color,
            range: 2.0,
            shadows: false,
        }
    }

    /// Spawn the fixture model (if any) and its point light.
    pub fn spawn(&self, commands: &mut Commands, asset_server: &AssetServer) {
        if !self.model.is_empty() {
            glb(commands, asset_server, self.model, self.model_pos, self.model_rot);
        }
        commands.spawn((
            PointLight {
                intensity: self.intensity,
                color: self.color,
                range: self.range,
                shadows_enabled: self.shadows,
                ..default()
            },
            Transform::from_translation(self.light_pos),
        ));
    }
}

pub fn spawn_box(
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

pub fn spawn_wall(
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

pub fn spawn_floor_ceiling(
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

pub fn spawn_grate_bars(
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

pub fn spawn_doorframe(
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

pub fn spawn_doorframe_x(
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

pub fn spawn_stairs(
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

/// Spawn a GLB scene at a given position and rotation.
pub fn glb(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: &str,
    pos: Vec3,
    rot: Quat,
) {
    let scene: Handle<Scene> = asset_server.load(format!("{path}#Scene0"));
    commands.spawn((
        SceneRoot(scene),
        Transform::from_translation(pos).with_rotation(rot),
    ));
}
