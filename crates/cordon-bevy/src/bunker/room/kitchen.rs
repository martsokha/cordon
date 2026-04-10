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
    prop(
        commands,
        asset_server,
        Prop::AmericanFridge,
        Vec3::new(l.kitchen_x_min() + 0.4, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Kitchen shelves as counter surface — centered on the far wall.
    // KitchenShelves1 top is at y = 0.865 (from measured AABB).
    const SHELF_SURFACE: f32 = 0.865;
    prop(
        commands,
        asset_server,
        Prop::KitchenShelves1,
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        commands,
        asset_server,
        Prop::KitchenShelves2,
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Items on the shelves — feet at shelf surface height.
    prop(
        commands,
        asset_server,
        Prop::Microwave,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        commands,
        asset_server,
        Prop::Kettle,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        commands,
        asset_server,
        Prop::Mug,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj_center() + 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Electric box mounted on the south wall.
    prop(
        commands,
        asset_server,
        Prop::ElectricBox01,
        Vec3::new(l.kitchen_x_center() + 0.5, 0.0, l.back_z + 0.05),
        Quat::IDENTITY,
    );

    // Barrel near the doorway.
    prop(
        commands,
        asset_server,
        Prop::Barrel02,
        Vec3::new(l.kitchen_x_center() + 0.8, 0.0, l.tj_north - 0.4),
        Quat::IDENTITY,
    );

    // Near the doorway: supply bag and a box.
    prop(
        commands,
        asset_server,
        Prop::Bag01,
        Vec3::new(l.kitchen_x_center() + 0.8, 0.0, l.tj_north - 0.3),
        Quat::from_rotation_y(0.6),
    );
    prop(
        commands,
        asset_server,
        Prop::Box01,
        Vec3::new(l.kitchen_x_center() + 0.3, 0.0, l.tj_north - 0.5),
        Quat::from_rotation_y(0.2),
    );
}
