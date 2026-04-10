//! Utility room (left side room): fridge, counter, microwave, kettle.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

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
    spawn_floor_ceiling(
        commands,
        meshes,
        pal.concrete_dark.clone(),
        Vec3::new(l.util_x_center, 0.0, l.tj_center),
        floor_half,
        l.h,
    );

    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.util_x_min, l.hh, l.tj_center),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(l.tj_len / 2.0, l.hh),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.util_x_center, l.hh, l.tj_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.util_x_center, l.hh, l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh),
    );

    // Fridge against the far wall, near the north corner.
    glb(
        commands,
        asset_server,
        "models/interior/AmericanFridge.glb",
        Vec3::new(l.util_x_min + 0.4, 0.0, l.tj_north - 0.4),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Two kitchen shelves side by side as a wider counter surface.
    glb(
        commands,
        asset_server,
        "models/interior/KitchenShelves1.glb",
        Vec3::new(l.util_x_min + 0.3, 0.0, l.tj_center - 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/interior/KitchenShelves2.glb",
        Vec3::new(l.util_x_min + 0.3, 0.0, l.tj_center + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Microwave on the north shelves (spaced from fridge).
    glb(
        commands,
        asset_server,
        "models/interior/Microwave.glb",
        Vec3::new(l.util_x_min + 0.4, 0.9, l.tj_center - 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Kettle on the south shelves.
    glb(
        commands,
        asset_server,
        "models/interior/Kettle.glb",
        Vec3::new(l.util_x_min + 0.4, 0.9, l.tj_center + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Mug between them.
    glb(
        commands,
        asset_server,
        "models/interior/Mug.glb",
        Vec3::new(l.util_x_min + 0.4, 0.9, l.tj_center + 0.1),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Electric box on the far wall.
    glb(
        commands,
        asset_server,
        "models/storage/ElectricBox_01.glb",
        Vec3::new(l.util_x_min + 0.05, 1.5, l.tj_center - 1.0),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Barrel in the back corner.
    glb(
        commands,
        asset_server,
        "models/storage/Barrel_02.glb",
        Vec3::new(l.util_x_center - 0.5, 0.0, l.back_z + 0.4),
        Quat::IDENTITY,
    );
}
