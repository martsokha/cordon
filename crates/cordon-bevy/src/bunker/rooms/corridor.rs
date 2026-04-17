//! Main corridor: floor, ceiling, front/back walls, and two side
//! walls with T1 + T2 doorway openings cut into them.
//!
//! Before this file existed, corridor geometry was baked into
//! `bunker/systems.rs::spawn_corridor`, which threaded
//! `(commands, meshes, pal, l)` through three levels. Moving it
//! into a regular `rooms/*` module makes corridor placement read
//! the same as every other room — one `ctx` parameter, same
//! geometry shortcuts, same ergonomics.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let concrete = ctx.pal.concrete.clone();
    let concrete_dark = ctx.pal.concrete_dark.clone();

    // Floor + ceiling span the full extended corridor.
    let main_center_z = (ctx.l.front_z + ctx.l.back_z) / 2.0;
    let main_floor_half = Vec2::new(ctx.l.hw, (ctx.l.front_z - ctx.l.back_z) / 2.0);
    ctx.floor_ceiling(
        Vec3::new(0.0, 0.0, main_center_z),
        main_floor_half,
        ctx.l.h,
        &concrete_dark,
    );

    // Front + back walls.
    ctx.wall(
        Vec3::new(0.0, ctx.l.hh(), ctx.l.front_z),
        Quat::from_rotation_y(PI),
        Vec2::new(ctx.l.hw, ctx.l.hh()),
        &concrete,
    );
    ctx.wall(
        Vec3::new(0.0, ctx.l.hh(), ctx.l.back_z),
        Quat::IDENTITY,
        Vec2::new(ctx.l.hw, ctx.l.hh()),
        &concrete,
    );

    // Side walls with T1 + T2 door openings.
    spawn_side_wall(ctx, -ctx.l.hw, Quat::from_rotation_y(FRAC_PI_2));
    spawn_side_wall(ctx, ctx.l.hw, Quat::from_rotation_y(-FRAC_PI_2));
}

/// One side of the main corridor: a wall running
/// `front_z → back_z` with T1 and T2 doorway openings cut into
/// it. Emits up to three wall segments (front stub, between-Ts,
/// back stub) plus a doorframe at each T centre.
///
/// `x` is the wall's X-coordinate (`-hw` for left, `+hw` for
/// right); `rot` orients the wall facing the corridor interior.
fn spawn_side_wall(ctx: &mut RoomCtx<'_, '_, '_>, x: f32, rot: Quat) {
    let l = ctx.l;
    let hh = l.hh();
    let gaps = [
        // Front stub: front_z → T1 door north edge.
        (l.front_z, l.tj1_center() + l.side_door_width / 2.0),
        // Between T1 door south edge and T2 door north edge.
        (
            l.tj1_center() - l.side_door_width / 2.0,
            l.tj2_center() + l.side_door_width / 2.0,
        ),
        // Back stub: T2 door south edge → back_z.
        (l.tj2_center() - l.side_door_width / 2.0, l.back_z),
    ];
    let concrete = ctx.pal.concrete.clone();
    for (n, s) in gaps {
        let len = (n - s).abs();
        if len <= 0.1 {
            continue;
        }
        let cz = (n + s) / 2.0;
        ctx.wall(
            Vec3::new(x, hh, cz),
            rot,
            Vec2::new(len / 2.0, hh),
            &concrete,
        );
    }

    // Doorframes at each T-junction centre.
    let tj1 = ctx.l.tj1_center();
    let tj2 = ctx.l.tj2_center();
    let width = ctx.l.side_door_width;
    let opening_h = ctx.l.opening_h();
    ctx.doorframe_x(x, tj1, width, opening_h, &concrete);
    ctx.doorframe_x(x, tj2, width, opening_h, &concrete);
}
