//! Utility room (left side room): fridge, shelves, microwave, kettle.

use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use super::geometry::*;
use super::{Layout, Palette};

pub fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    l: &Layout,
) {
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj_len / 2.0);
    spawn_floor_ceiling(commands, meshes, pal.concrete_dark.clone(),
        Vec3::new(l.util_x_center, 0.0, l.tj_center), floor_half, l.h);

    // Far wall (west).
    spawn_wall(commands, meshes, pal.concrete.clone(),
        Vec3::new(l.util_x_min, l.hh, l.tj_center), Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(l.tj_len / 2.0, l.hh));
    // North wall.
    spawn_wall(commands, meshes, pal.concrete.clone(),
        Vec3::new(l.util_x_center, l.hh, l.tj_north), Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh));
    // South wall.
    spawn_wall(commands, meshes, pal.concrete.clone(),
        Vec3::new(l.util_x_center, l.hh, l.back_z), Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh));

    // Fridge.
    glb(commands, asset_server, "models/interior/AmericanFridge.glb",
        Vec3::new(l.util_x_min + 0.4, 0.0, l.tj_north - 0.4), Quat::from_rotation_y(FRAC_PI_2));
    // Kitchen shelves.
    glb(commands, asset_server, "models/interior/KitchenShelves1.glb",
        Vec3::new(l.util_x_min + 0.3, 0.0, l.tj_center), Quat::from_rotation_y(FRAC_PI_2));
    // Microwave (away from fridge).
    glb(commands, asset_server, "models/interior/Microwave.glb",
        Vec3::new(l.util_x_min + 0.4, 0.9, l.tj_center - 0.3), Quat::from_rotation_y(FRAC_PI_2));
    // Kettle.
    glb(commands, asset_server, "models/interior/Kettle.glb",
        Vec3::new(l.util_x_min + 0.4, 0.9, l.tj_center + 0.3), Quat::from_rotation_y(FRAC_PI_2));
}
