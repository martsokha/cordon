//! Entry checkpoint zone: stairs, trade grate, lockers, visitor side.

use bevy::prelude::*;

use super::geometry::*;
use super::{Layout, Palette};

pub fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    pal: &Palette,
    l: &Layout,
) {
    spawn_doorframe(commands, meshes, pal.concrete.clone(), 0.0, l.front_z - 0.1, 1.0);
    spawn_stairs(commands, meshes, pal.concrete.clone(), l.front_z + 0.3, 1.0, 6);

    // Trade grate: sides + bars below counter to block walking.
    spawn_grate_bars(commands, meshes, pal.metal.clone(), -l.hw, -l.hole_half, l.trade_z, l.h, 0.1);
    spawn_grate_bars(commands, meshes, pal.metal.clone(), l.hole_half, l.hw, l.trade_z, l.h, 0.1);
    spawn_box(commands, meshes, pal.wood.clone(),
        Vec3::new(0.0, 0.78, l.trade_z), Vec3::new(l.hole_half * 2.0 + 0.2, 0.04, 0.25));
    spawn_grate_bars(commands, meshes, pal.metal.clone(), -l.hole_half, l.hole_half, l.trade_z, 0.76, 0.1);
    // Invisible full-height collider across the center opening so
    // the player can't step over the short bars.
    commands.spawn((
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(l.hole_half * 2.0, l.h, 0.1),
        Transform::from_xyz(0.0, l.h / 2.0, l.trade_z),
    ));

    glb(commands, asset_server, "models/interior/WoodenStool.glb",
        Vec3::new(0.0, 0.0, l.trade_z + 0.6), Quat::IDENTITY);
    // Lockers along the left wall.
    for i in 0..5 {
        glb(commands, asset_server, "models/storage/Locker.glb",
            Vec3::new(-l.hw + 0.3, 0.0, 2.2 + 0.5 * i as f32), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
    }
    spawn_box(commands, meshes, mats.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.17),
        perceptual_roughness: 0.6, metallic: 0.5, ..default()
    }), Vec3::new(l.hw - 0.4, 0.3, 3.0), Vec3::new(0.7, 0.6, 1.0));
}
