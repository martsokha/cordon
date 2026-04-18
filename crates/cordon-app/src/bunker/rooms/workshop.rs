//! Workshop (right side of T2): generator, toolbox, tinkering
//! bench. Lived-in and industrial.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let concrete = ctx.pal.concrete.clone();
    let concrete_dark = ctx.pal.concrete_dark.clone();

    ctx.floor_ceiling(
        Vec3::new(ctx.l.workshop_x_center(), 0.0, ctx.l.tj2_center()),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.tj2_len() / 2.0),
        ctx.l.h,
        &concrete_dark,
    );
    ctx.wall(
        Vec3::new(ctx.l.workshop_x_max(), ctx.l.hh(), ctx.l.tj2_center()),
        Quat::from_rotation_y(FRAC_PI_2),
        Vec2::new(ctx.l.tj2_len() / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.workshop_x_center(), ctx.l.hh(), ctx.l.tj2_north),
        Quat::from_rotation_y(PI),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.workshop_x_center(), ctx.l.hh(), ctx.l.back_z),
        Quat::IDENTITY,
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );

    // Generator against the far wall. The prop's native X span
    // reaches ~1.04 on the negative side; with the -π/2 Y rotation
    // that becomes the world-Z half-extent, so it has to sit at
    // least 1.04 away from the back wall to avoid clipping.
    ctx.prop_rot(
        Prop::Generator1,
        Vec3::new(ctx.l.workshop_x_max() - 0.6, 0.0, ctx.l.back_z + 1.1),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    ctx.prop_rot(
        Prop::Toolbox1,
        Vec3::new(ctx.l.workshop_x_center() + 0.3, 0.0, ctx.l.tj2_north - 0.4),
        Quat::from_rotation_y(0.3),
    );
    ctx.prop_rot(
        Prop::Bucket1,
        Vec3::new(
            ctx.l.workshop_x_center() - 0.4,
            0.0,
            ctx.l.tj2_center() + 0.5,
        ),
        Quat::from_rotation_y(0.2),
    );
    ctx.prop_rot(
        Prop::Chair,
        Vec3::new(
            ctx.l.workshop_x_center() + 0.2,
            0.0,
            ctx.l.tj2_center() - 0.2,
        ),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    ctx.prop(
        Prop::FireExtinguisher,
        Vec3::new(ctx.l.workshop_x_center() - 0.8, 0.0, ctx.l.tj2_north - 0.4),
    );
}
