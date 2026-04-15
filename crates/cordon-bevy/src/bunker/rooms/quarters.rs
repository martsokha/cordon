//! Quarters (right side room): sofa, pillow, rug, personal items.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let concrete = ctx.pal.concrete.clone();
    let concrete_dark = ctx.pal.concrete_dark.clone();

    ctx.floor_ceiling(
        Vec3::new(ctx.l.quarters_x_center(), 0.0, ctx.l.tj1_center()),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.tj1_len() / 2.0),
        ctx.l.h,
        &concrete_dark,
    );
    ctx.wall(
        Vec3::new(ctx.l.quarters_x_max(), ctx.l.hh(), ctx.l.tj1_center()),
        Quat::from_rotation_y(FRAC_PI_2),
        Vec2::new(ctx.l.tj1_len() / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.quarters_x_center(), ctx.l.hh(), ctx.l.tj1_north),
        Quat::from_rotation_y(PI),
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(ctx.l.quarters_x_center(), ctx.l.hh(), ctx.l.tj1_south),
        Quat::IDENTITY,
        Vec2::new(ctx.l.side_depth / 2.0, ctx.l.hh()),
        &concrete,
    );

    // Wide sofa against the far wall. Cushion top at y = 0.4.
    const SOFA_CUSHION: f32 = 0.4;
    ctx.prop_rot(
        Prop::WideSofa,
        Vec3::new(ctx.l.quarters_x_max() - 0.5, 0.0, ctx.l.tj1_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    ctx.prop(
        Prop::Pillow,
        Vec3::new(
            ctx.l.quarters_x_max() - 0.5,
            SOFA_CUSHION,
            ctx.l.tj1_center() + 0.5,
        ),
    );
    ctx.prop(
        Prop::Rug,
        Vec3::new(ctx.l.quarters_x_center(), 0.0, ctx.l.tj1_center()),
    );
    ctx.prop_rot(
        Prop::SingleBookshelf,
        Vec3::new(ctx.l.quarters_x_max() - 0.3, 0.0, ctx.l.tj1_south + 0.3),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    ctx.prop_rot(
        Prop::Suitcase01,
        Vec3::new(ctx.l.quarters_x_center() - 0.3, 0.0, ctx.l.tj1_south + 0.3),
        Quat::from_rotation_y(0.4),
    );
    ctx.prop(
        Prop::Lamp1,
        Vec3::new(ctx.l.quarters_x_max() - 0.3, 0.0, ctx.l.tj1_center() - 0.8),
    );
    ctx.prop(
        Prop::PlantPot2,
        Vec3::new(ctx.l.hw + 0.4, 0.0, ctx.l.tj1_south + 0.3),
    );
    ctx.prop_rot(
        Prop::Pillow,
        Vec3::new(
            ctx.l.quarters_x_max() - 0.5,
            SOFA_CUSHION,
            ctx.l.tj1_center() - 0.3,
        ),
        Quat::from_rotation_y(0.5),
    );
    // PlantPot1 top is at y = 0.48 (measured).
    const POT1_TOP: f32 = 0.480;
    ctx.prop(
        Prop::PlantPot1,
        Vec3::new(ctx.l.hw + 0.4, 0.0, ctx.l.tj1_center() + 1.0),
    );
    ctx.prop(
        Prop::Cactus,
        Vec3::new(ctx.l.hw + 0.4, POT1_TOP, ctx.l.tj1_center() + 1.0),
    );
}
