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

    // Wall-mounted diagnostic machine.
    ctx.prop_rot(
        Prop::WallMachine,
        Vec3::new(ctx.l.infirmary_x_min() + 0.1, 0.9, ctx.l.tj2_center() - 0.6),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Medication + syringe on the floor.
    ctx.prop(
        Prop::MedicationCluster1,
        Vec3::new(ctx.l.infirmary_x_min() + 0.3, 0.0, ctx.l.tj2_center() + 0.5),
    );
    ctx.prop(
        Prop::MedicationBottle,
        Vec3::new(
            ctx.l.infirmary_x_min() + 0.25,
            0.0,
            ctx.l.tj2_center() + 0.2,
        ),
    );
    ctx.prop_rot(
        Prop::Syringe,
        Vec3::new(ctx.l.infirmary_x_min() + 0.4, 0.0, ctx.l.tj2_center() + 0.7),
        Quat::from_rotation_y(0.6),
    );
    ctx.prop_rot(
        Prop::BreathingAparatus,
        Vec3::new(ctx.l.infirmary_x_min() + 0.35, 0.0, ctx.l.back_z + 0.4),
        Quat::from_rotation_y(0.4),
    );

    // Stool for the patient.
    ctx.prop(
        Prop::WoodenStool,
        Vec3::new(ctx.l.infirmary_x_center() + 0.2, 0.0, ctx.l.tj2_center()),
    );

    // Face masks by the door.
    ctx.prop_rot(
        Prop::FaceMask1,
        Vec3::new(ctx.l.infirmary_x_center() + 0.8, 0.0, ctx.l.tj2_north - 0.3),
        Quat::from_rotation_y(0.8),
    );
    ctx.prop_rot(
        Prop::FaceMask2,
        Vec3::new(ctx.l.infirmary_x_center() + 0.9, 0.0, ctx.l.tj2_north - 0.5),
        Quat::from_rotation_y(1.2),
    );

    // Paper trash cluster in the corner.
    ctx.prop_rot(
        Prop::PaperTrashCluster1,
        Vec3::new(ctx.l.infirmary_x_center() - 0.4, 0.0, ctx.l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );
}
