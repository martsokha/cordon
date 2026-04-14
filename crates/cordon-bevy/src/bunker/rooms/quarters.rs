//! Quarters (right side room): sofa, pillow, rug, personal items.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj1_len() / 2.0);
    spawn_floor_ceiling(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete_dark.clone(),
        Vec3::new(l.quarters_x_center(), 0.0, l.tj1_center()),
        floor_half,
        l.h,
    );

    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.quarters_x_max(), l.hh(), l.tj1_center()),
        Quat::from_rotation_y(FRAC_PI_2),
        Vec2::new(l.tj1_len() / 2.0, l.hh()),
    );
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.quarters_x_center(), l.hh(), l.tj1_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.quarters_x_center(), l.hh(), l.tj1_south),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Wide sofa against the far wall. Cushion top is around y = 0.4.
    const SOFA_CUSHION: f32 = 0.4;
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::WideSofa,
        Vec3::new(l.quarters_x_max() - 0.5, 0.0, l.tj1_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Pillow on the sofa cushion.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Pillow,
        Vec3::new(l.quarters_x_max() - 0.5, SOFA_CUSHION, l.tj1_center() + 0.5),
        Quat::IDENTITY,
    );
    // Rug.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Rug,
        Vec3::new(l.quarters_x_center(), 0.0, l.tj1_center()),
        Quat::IDENTITY,
    );
    // Small bookshelf (personal books).
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::SingleBookshelf,
        Vec3::new(l.quarters_x_max() - 0.3, 0.0, l.tj1_south + 0.3),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Suitcase — personal belongings.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Suitcase01,
        Vec3::new(l.quarters_x_center() - 0.3, 0.0, l.tj1_south + 0.3),
        Quat::from_rotation_y(0.4),
    );
    // Lamp next to the sofa.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Lamp1,
        Vec3::new(l.quarters_x_max() - 0.3, 0.0, l.tj1_center() - 0.8),
        Quat::IDENTITY,
    );
    // A bit of life.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::PlantPot2,
        Vec3::new(l.hw + 0.4, 0.0, l.tj1_south + 0.3),
        Quat::IDENTITY,
    );
    // Second pillow.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Pillow,
        Vec3::new(l.quarters_x_max() - 0.5, SOFA_CUSHION, l.tj1_center() - 0.3),
        Quat::from_rotation_y(0.5),
    );
    // Cactus in a pot — the one living thing down here.
    // PlantPot1 top is at y = 0.48 (measured).
    const POT1_TOP: f32 = 0.480;
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::PlantPot1,
        Vec3::new(l.hw + 0.4, 0.0, l.tj1_center() + 1.0),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Cactus,
        Vec3::new(l.hw + 0.4, POT1_TOP, l.tj1_center() + 1.0),
        Quat::IDENTITY,
    );
}
