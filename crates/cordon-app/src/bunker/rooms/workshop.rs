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
    // Diagnostic machine on the back wall next to the
    // generator. `Rx(π/2)` stands it upright with the panel
    // face pointing into the room; the leading `Rz(π)` spins
    // it 180° in the wall plane (around the wall normal) so
    // the panel's top/bottom are swapped without leaving the
    // wall.
    ctx.prop_rot(
        Prop::WallMachine,
        Vec3::new(ctx.l.workshop_x_center() - 0.5, 1.25, ctx.l.back_z + 0.1),
        Quat::from_rotation_z(PI) * Quat::from_rotation_x(FRAC_PI_2),
    );
    // Wall-mounted lowpoly extinguisher. Mounted on the east
    // wall at ~1 m up; `Extinguisher`'s AABB is anchored at the
    // bracket so the mesh sits to the +x side of its origin.
    ctx.prop_rot(
        Prop::Extinguisher,
        Vec3::new(ctx.l.workshop_x_max() - 0.05, 0.9, ctx.l.tj2_center() + 0.8),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Floor-standing electrical panel against the north wall
    // (opposite the generator's back wall), door-side of the
    // room so you see it on the way in. Faces south into the
    // room so the breaker panel reads on entry.
    ctx.prop_rot(
        Prop::ElectricBox01,
        Vec3::new(ctx.l.workshop_x_center() + 0.5, 0.0, ctx.l.tj2_north - 0.3),
        Quat::from_rotation_y(PI),
    );
    // Storage crate against the east wall near the generator.
    ctx.prop_rot(
        Prop::StorageCrate010,
        Vec3::new(ctx.l.workshop_x_max() - 0.35, 0.0, ctx.l.back_z + 0.45),
        Quat::from_rotation_y(0.3),
    );
}
