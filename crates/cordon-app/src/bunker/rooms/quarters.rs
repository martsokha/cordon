//! Quarters (right side room): sofa, pillow, rug, personal items.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

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

    // Wide sofa against the far wall, centred on the room's Z.
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
    ctx.prop_rot(
        Prop::Pillow,
        Vec3::new(
            ctx.l.quarters_x_max() - 0.5,
            SOFA_CUSHION,
            ctx.l.tj1_center() - 0.3,
        ),
        Quat::from_rotation_y(0.5),
    );
    ctx.prop(
        Prop::Rug,
        Vec3::new(ctx.l.quarters_x_center(), 0.0, ctx.l.tj1_center()),
    );

    // Bookshelf against the south wall (not the far wall where
    // the sofa is — avoids the overlap).
    let bookshelf_x = ctx.l.quarters_x_center() + 0.3;
    let bookshelf_z = ctx.l.tj1_south + 0.25;
    const BOOKSHELF_TOP: f32 = 0.4;
    ctx.prop_rot(
        Prop::SingleBookshelf,
        Vec3::new(bookshelf_x, 0.0, bookshelf_z),
        Quat::IDENTITY,
    );
    // Medical masks sitting on top of the bookshelf.
    ctx.prop_rot(
        Prop::FaceMask1,
        Vec3::new(bookshelf_x - 0.25, BOOKSHELF_TOP, bookshelf_z),
        Quat::from_rotation_y(0.8),
    );
    ctx.prop_rot(
        Prop::FaceMask2,
        Vec3::new(bookshelf_x + 0.2, BOOKSHELF_TOP, bookshelf_z + 0.05),
        Quat::from_rotation_y(1.2),
    );
    // Paper stack + alarm clock also on the bookshelf, offset
    // so they don't stack onto the masks.
    ctx.prop_rot(
        Prop::PaperStack01,
        Vec3::new(bookshelf_x + 0.45, BOOKSHELF_TOP, bookshelf_z - 0.02),
        Quat::from_rotation_y(0.2),
    );
    ctx.prop_rot(
        Prop::AlarmClock01,
        Vec3::new(bookshelf_x - 0.55, BOOKSHELF_TOP, bookshelf_z + 0.02),
        Quat::from_rotation_y(-FRAC_PI_4),
    );

    // Suitcase in the south-west corner.
    ctx.prop_rot(
        Prop::Suitcase01,
        Vec3::new(ctx.l.hw + 0.4, 0.0, ctx.l.tj1_south + 0.3),
        Quat::from_rotation_y(0.4),
    );

    // Coffee table rotated 90° and pulled away from the sofa.
    const TABLE_TOP: f32 = 0.4;
    let table_x = ctx.l.quarters_x_center() + 0.2;
    let table_z = ctx.l.tj1_center();
    ctx.prop_rot(
        Prop::ModernCoffeeTable,
        Vec3::new(table_x, 0.0, table_z),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    // Medication props on the coffee table.
    ctx.prop(
        Prop::MedicationCluster1,
        Vec3::new(table_x - 0.05, TABLE_TOP, table_z + 0.1),
    );
    ctx.prop(
        Prop::MedicationBottle,
        Vec3::new(table_x + 0.1, TABLE_TOP, table_z - 0.08),
    );
    ctx.prop_rot(
        Prop::Syringe,
        Vec3::new(table_x + 0.2, TABLE_TOP, table_z + 0.12),
        Quat::from_rotation_y(0.6),
    );
    ctx.prop(
        Prop::Bottles01,
        Vec3::new(table_x - 0.15, TABLE_TOP, table_z - 0.05),
    );

    // PlantPot2 (the nicer pot) with cactus, near the corridor
    // entrance so it greets you as you walk in.
    const POT2_TOP: f32 = 0.488;
    ctx.prop(
        Prop::PlantPot2,
        Vec3::new(ctx.l.hw + 0.4, 0.0, ctx.l.tj1_north - 0.4),
    );
    ctx.prop(
        Prop::Cactus,
        Vec3::new(ctx.l.hw + 0.4, POT2_TOP, ctx.l.tj1_north - 0.4),
    );
}
