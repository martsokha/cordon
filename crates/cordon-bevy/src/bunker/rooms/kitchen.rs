//! Kitchen (left side room): fridge, counter, microwave, kettle.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::particles;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj1_len() / 2.0);
    spawn_floor_ceiling(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete_dark.clone(),
        Vec3::new(l.kitchen_x_center(), 0.0, l.tj1_center()),
        floor_half,
        l.h,
    );

    // Walls.
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.kitchen_x_min(), l.hh(), l.tj1_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(l.tj1_len() / 2.0, l.hh()),
    );
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.kitchen_x_center(), l.hh(), l.tj1_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.kitchen_x_center(), l.hh(), l.tj1_south),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Fridge against the far wall, near the south corner.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::AmericanFridge,
        Vec3::new(l.kitchen_x_min() + 0.4, 0.0, l.tj1_south + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Kitchen shelves as counter surface — centered on the far wall.
    // KitchenShelves1 top is at y = 0.865 (from measured AABB).
    const SHELF_SURFACE: f32 = 0.865;
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::KitchenShelves1,
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj1_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::KitchenShelves2,
        Vec3::new(l.kitchen_x_min() + 0.3, 0.0, l.tj1_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Items on the shelves — feet at shelf surface height.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Microwave,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj1_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    let kettle = prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Kettle,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj1_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Steam rising from the spout. Offset is in the kettle's
    // *local* frame; the kettle is rotated +90° around Y, so
    // local +Z maps to world +X — the direction the nose points
    // (toward the player entering the kitchen from the corridor).
    particles::attach_kettle_steam(ctx.commands, ctx.effects, kettle, Vec3::new(0.0, 0.3, 0.1));
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Mug,
        Vec3::new(l.kitchen_x_min() + 0.4, SHELF_SURFACE, l.tj1_center() + 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Electric box mounted on the south wall.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::ElectricBox01,
        Vec3::new(l.kitchen_x_center() + 0.5, 0.0, l.tj1_south + 0.05),
        Quat::IDENTITY,
    );

    // Barrel near the doorway.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Barrel02,
        Vec3::new(l.kitchen_x_center() + 0.8, 0.0, l.tj1_north - 0.4),
        Quat::IDENTITY,
    );

    // Near the doorway: supply bag and a box.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Bag01,
        Vec3::new(l.kitchen_x_center() + 0.8, 0.0, l.tj1_north - 0.3),
        Quat::from_rotation_y(0.6),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(l.kitchen_x_center() + 0.3, 0.0, l.tj1_north - 0.5),
        Quat::from_rotation_y(0.2),
    );
}
