//! Kitchen (left side room): fridge, counter, microwave, kettle.

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
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj_len() / 2.0);
    spawn_floor_ceiling(
        commands,
        meshes,
        pal.concrete_dark.clone(),
        Vec3::new(l.kitchen_x_center(), 0.0, l.tj_center()),
        floor_half,
        l.h,
    );

    // Walls.
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.kitchen_x_min(), l.hh(), l.tj_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(l.tj_len() / 2.0, l.hh()),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.kitchen_x_center(), l.hh(), l.tj_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.kitchen_x_center(), l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Fridge against the far wall, near the south corner.
    // Positioned further from the shelves to avoid clipping.
    glb(
        commands,
        asset_server,
        "models/interior/AmericanFridge.glb",
        Vec3::new(l.kitchen_x_min() + 0.4, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Kitchen shelves as counter surface — centered on the far wall.
    let shelf_surface = 0.86; // KitchenShelves1 top is at y=0.865
    glb(
        commands,
        asset_server,
        "models/interior/KitchenShelves1.glb",
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/interior/KitchenShelves2.glb",
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Items on the shelves — at shelf surface height.
    glb(
        commands,
        asset_server,
        "models/interior/Microwave.glb",
        Vec3::new(l.kitchen_x_min() + 0.4, shelf_surface, l.tj_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/interior/Kettle.glb",
        Vec3::new(l.kitchen_x_min() + 0.4, shelf_surface, l.tj_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/interior/Mug.glb",
        Vec3::new(l.kitchen_x_min() + 0.4, shelf_surface, l.tj_center() + 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Electric box on the south wall at reasonable height.
    // Storage pack origins are at center, so offset by half-height.
    glb(
        commands,
        asset_server,
        "models/storage/ElectricBox_01.glb",
        Vec3::new(l.kitchen_x_center() + 0.5, 0.2, l.back_z + 0.05),
        Quat::IDENTITY,
    );

    // Barrel — storage pack origin at center, half-height ~0.21.
    glb(
        commands,
        asset_server,
        "models/storage/Barrel_02.glb",
        Vec3::new(l.kitchen_x_center() + 0.8, 0.22, l.tj_north - 0.4),
        Quat::IDENTITY,
    );

    // Near the doorway: supply bag and a box.
    glb(
        commands,
        asset_server,
        "models/storage/Bag_01.glb",
        Vec3::new(l.kitchen_x_center() + 0.8, 0.1, l.tj_north - 0.3),
        Quat::from_rotation_y(0.6),
    );
    glb(
        commands,
        asset_server,
        "models/storage/Box_01.glb",
        Vec3::new(l.kitchen_x_center() + 0.3, 0.15, l.tj_north - 0.5),
        Quat::from_rotation_y(0.2),
    );
}
