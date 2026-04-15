//! Workshop (right side of T2): generator, toolbox, tinkering
//! bench. Feels lived-in and industrial — this is where things
//! get fixed.

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
        Vec3::new(l.workshop_x_center(), 0.0, l.tj2_center()),
        floor_half,
        l.h,
    );

    // Outer wall (+x).
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.workshop_x_max(), l.hh(), l.tj2_center()),
        Quat::from_rotation_y(FRAC_PI_2),
        Vec2::new(l.tj2_len() / 2.0, l.hh()),
    );
    // North wall.
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.workshop_x_center(), l.hh(), l.tj2_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    // South wall (= bunker back wall for this side).
    spawn_wall(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        Vec3::new(l.workshop_x_center(), l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Generator against the far wall — the reason the bunker has
    // power (metaphorically; the `upgrade_generator` consequence is
    // what actually gates power in-sim).
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Generator1,
        Vec3::new(l.workshop_x_max() - 0.6, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(-FRAC_PI_2),
    );

    // Toolbox on the floor near the door.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Toolbox1,
        Vec3::new(l.workshop_x_center() + 0.3, 0.0, l.tj2_north - 0.4),
        Quat::from_rotation_y(0.3),
    );

    // Bucket on the floor near the bench. The rack + radio that
    // used to sit here moved out: the rack because the scene was
    // getting cluttered, the radio to the command desk where
    // it's in the player's line of sight.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Bucket1,
        Vec3::new(l.workshop_x_center() - 0.4, 0.0, l.tj2_center() + 0.5),
        Quat::from_rotation_y(0.2),
    );

    // Chair at the workbench area.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Chair,
        Vec3::new(l.workshop_x_center() + 0.2, 0.0, l.tj2_center() - 0.2),
        Quat::from_rotation_y(-FRAC_PI_2),
    );

    // Fire extinguisher mounted by the door (workshop = fire risk).
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::FireExtinguisher,
        Vec3::new(l.workshop_x_center() - 0.8, 0.0, l.tj2_north - 0.4),
        Quat::IDENTITY,
    );
}
