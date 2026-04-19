//! Infirmary (left side of T2): medical bay with bottles, masks,
//! breathing apparatus, and a stool for the patient.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let concrete = ctx.pal.concrete.clone();
    let concrete_dark = ctx.pal.concrete_dark.clone();

    ctx.floor_ceiling(
        Vec3::new(ctx.l.infirmary_x_center(), 0.0, ctx.l.tj2_center()),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.tj2_len() / 2.0),
        ctx.l.h,
        &concrete_dark,
    );
    ctx.wall(
        Vec3::new(ctx.l.infirmary_x_min(), ctx.l.hh(), ctx.l.tj2_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(ctx.l.tj2_len() / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.infirmary_x_center(), ctx.l.hh(), ctx.l.tj2_north),
        Quat::from_rotation_y(PI),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.infirmary_x_center(), ctx.l.hh(), ctx.l.back_z),
        Quat::IDENTITY,
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );

    // Breathing apparatus in the corner. Decorative only — the
    // takeable pill cluster lives in the quarters.
    ctx.prop_rot(
        Prop::BreathingAparatus,
        Vec3::new(ctx.l.infirmary_x_min() + 0.35, 0.0, ctx.l.back_z + 0.4),
        Quat::from_rotation_y(0.4),
    );

    // Paper trash cluster in the corner.
    ctx.prop_rot(
        Prop::PaperTrashCluster1,
        Vec3::new(ctx.l.infirmary_x_center() - 0.4, 0.0, ctx.l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );
}
