//! Kitchen (left side room): fridge, counter, microwave, kettle.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::particles;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let concrete = ctx.pal.concrete.clone();
    let concrete_dark = ctx.pal.concrete_dark.clone();

    ctx.floor_ceiling(
        Vec3::new(ctx.l.kitchen_x_center(), 0.0, ctx.l.tj1_center()),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.tj1_len() / 2.0),
        ctx.l.h,
        &concrete_dark,
    );
    ctx.wall(
        Vec3::new(ctx.l.kitchen_x_min(), ctx.l.hh(), ctx.l.tj1_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(ctx.l.tj1_len() / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.kitchen_x_center(), ctx.l.hh(), ctx.l.tj1_north),
        Quat::from_rotation_y(PI),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.kitchen_x_center(), ctx.l.hh(), ctx.l.tj1_south),
        Quat::IDENTITY,
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );

    // Fridge.
    ctx.prop_rot(
        Prop::AmericanFridge,
        Vec3::new(ctx.l.kitchen_x_min() + 0.4, 0.0, ctx.l.tj1_south + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Kitchen shelves as counter surface. Top at y = 0.865.
    const SHELF_SURFACE: f32 = 0.865;
    ctx.prop_rot(
        Prop::KitchenShelves1,
        Vec3::new(ctx.l.kitchen_x_min() + 0.3, 0.0, ctx.l.tj1_center() - 0.3),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    ctx.prop_rot(
        Prop::KitchenShelves2,
        Vec3::new(ctx.l.kitchen_x_min() + 0.3, 0.0, ctx.l.tj1_center() + 0.7),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Items on the shelves.
    ctx.prop_rot(
        Prop::Microwave,
        Vec3::new(
            ctx.l.kitchen_x_min() + 0.4,
            SHELF_SURFACE,
            ctx.l.tj1_center() - 0.3,
        ),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    let kettle = ctx
        .prop_rot(
            Prop::Kettle,
            Vec3::new(
                ctx.l.kitchen_x_min() + 0.4,
                SHELF_SURFACE,
                ctx.l.tj1_center() + 0.7,
            ),
            Quat::from_rotation_y(FRAC_PI_2),
        )
        .id();
    // Steam rising from the spout. Offset is in the kettle's local
    // frame; the kettle is rotated +90° around Y, so local +Z maps
    // to world +X (the nose direction).
    particles::steam::attach_kettle_steam(
        ctx.commands,
        ctx.effects,
        kettle,
        Vec3::new(0.0, 0.3, 0.1),
    );
    ctx.prop_rot(
        Prop::Mug,
        Vec3::new(
            ctx.l.kitchen_x_min() + 0.4,
            SHELF_SURFACE,
            ctx.l.tj1_center() + 0.3,
        ),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Electric box on the south wall.
    ctx.prop(
        Prop::ElectricBox01,
        Vec3::new(ctx.l.kitchen_x_center() + 0.5, 0.0, ctx.l.tj1_south + 0.05),
    );

    // Near the doorway.
    ctx.prop(
        Prop::Barrel02,
        Vec3::new(ctx.l.kitchen_x_center() + 0.8, 0.0, ctx.l.tj1_north - 0.4),
    );
    ctx.prop_rot(
        Prop::Bag01,
        Vec3::new(ctx.l.kitchen_x_center() + 0.8, 0.0, ctx.l.tj1_north - 0.3),
        Quat::from_rotation_y(0.6),
    );
}
