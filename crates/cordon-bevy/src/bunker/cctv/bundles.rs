use bevy::prelude::*;

use super::components::CctvMonitor;
use super::materials::CctvMaterial;

pub fn spawn_monitor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_materials: &mut Assets<StandardMaterial>,
    cctv_materials: &mut Assets<CctvMaterial>,
    image: Handle<Image>,
    monitor_pos: Vec3,
    monitor_target: Vec3,
) {
    let screen_mat = cctv_materials.add(CctvMaterial::new(image, 1.0));

    let bezel_mat = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.06),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.36, 0.22, 0.05))),
        MeshMaterial3d(bezel_mat),
        Transform::from_translation(monitor_pos).looking_at(monitor_target, Vec3::Y),
    ));

    let dir = (monitor_target - monitor_pos).normalize_or_zero();
    let mut monitor_transform =
        Transform::from_translation(monitor_pos + dir * 0.03).looking_at(monitor_target, Vec3::Y);
    monitor_transform.scale.x = -1.0;
    commands.spawn((
        CctvMonitor,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.16, 0.09)))),
        MeshMaterial3d(screen_mat),
        monitor_transform,
    ));
}
