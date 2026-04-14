//! Infirmary (left side of T2): medical bay with bottles, masks,
//! breathing apparatus, and a stool for the patient.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj2_len() / 2.0);
    spawn_floor_ceiling(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete_dark.clone(),
        Vec3::new(l.infirmary_x_center(), 0.0, l.tj2_center()),
        floor_half,
        l.h,
    );

    // Outer wall (-x).
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.infirmary_x_min(), l.hh(), l.tj2_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
        Vec2::new(l.tj2_len() / 2.0, l.hh()),
    );
    // North wall (closes off the T2 side room from the corridor
    // strip between T1 and T2).
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.infirmary_x_center(), l.hh(), l.tj2_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    // South wall (= bunker back wall for this side).
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.infirmary_x_center(), l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Wall-mounted machine (diagnostic console) on the outer wall,
    // at waist height. Faces into the room (+x).
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::WallMachine,
        Vec3::new(l.infirmary_x_min() + 0.1, 0.9, l.tj2_center() - 0.6),
        Quat::from_rotation_y(FRAC_PI_2),
    );

    // Storage rack against the outer wall, further south, holds
    // medication + breathing gear.
    const RACK_SHELF: f32 = 0.572;
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::StorageRack01,
        Vec3::new(l.infirmary_x_min() + 0.3, 0.0, l.tj2_center() + 0.5),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::MedicationCluster1,
        Vec3::new(
            l.infirmary_x_min() + 0.3,
            RACK_SHELF,
            l.tj2_center() + 0.5,
        ),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::MedicationBottle,
        Vec3::new(
            l.infirmary_x_min() + 0.25,
            RACK_SHELF,
            l.tj2_center() + 0.2,
        ),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Syringe,
        Vec3::new(
            l.infirmary_x_min() + 0.4,
            RACK_SHELF,
            l.tj2_center() + 0.7,
        ),
        Quat::from_rotation_y(0.6),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::BreathingAparatus,
        Vec3::new(l.infirmary_x_min() + 0.35, 0.0, l.back_z + 0.4),
        Quat::from_rotation_y(0.4),
    );

    // Stool for the patient, centred in the room.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::WoodenStool,
        Vec3::new(l.infirmary_x_center() + 0.2, 0.0, l.tj2_center()),
        Quat::IDENTITY,
    );

    // Face masks in a small pile on the floor by the door.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::FaceMask1,
        Vec3::new(l.infirmary_x_center() + 0.8, 0.0, l.tj2_north - 0.3),
        Quat::from_rotation_y(0.8),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::FaceMask2,
        Vec3::new(l.infirmary_x_center() + 0.9, 0.0, l.tj2_north - 0.5),
        Quat::from_rotation_y(1.2),
    );

    // Paper trash cluster in the corner.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::PaperTrashCluster1,
        Vec3::new(l.infirmary_x_center() - 0.4, 0.0, l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );
}
