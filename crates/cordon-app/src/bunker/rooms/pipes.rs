//! Ceiling pipes. Decorative industrial detail made of atomic-
//! pack pipe pieces composited into runs under the corridor
//! ceiling on both sides of the main hall.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

/// Uniform scale on every pipe piece.
const PIPE_SCALE: f32 = 0.25;

/// Length of one `Pipe1Long` piece after [`PIPE_SCALE`].
const PIPE_PIECE_LEN: f32 = 3.0 * PIPE_SCALE;

/// Straight segments before the pipe bends into the wall.
const STRAIGHT_SEGMENTS: u32 = 3;

/// How far under the ceiling the pipe centreline hangs.
const PIPE_HANG: f32 = 0.24;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    spawn_run(ctx, -1.3, false);
    spawn_run(ctx, 1.3, true);
}

/// Spawn one pipe run hugging a side wall. `mirror` flips the
/// corner-bend rotation so the arm bends up into the ceiling on
/// the correct side when the run sits on the +X wall.
fn spawn_run(ctx: &mut RoomCtx<'_, '_, '_>, wall_x: f32, mirror: bool) {
    let start_z = ctx.l.back_z;
    // `Pipe1Long`'s long axis is native Y. Rotating +90° around X
    // sends local +Y to world +Z (horizontal along the corridor).
    let straight_rot = Quat::from_rotation_x(FRAC_PI_2);
    let y = ctx.l.h - PIPE_HANG;

    for i in 0..STRAIGHT_SEGMENTS {
        let z = start_z + (i as f32) * PIPE_PIECE_LEN;
        ctx.prop_placement(
            PropPlacement::new(Prop::Pipe1Long, Vec3::new(wall_x, y, z))
                .rotated(straight_rot)
                .scaled(PIPE_SCALE)
                .no_collider(),
        );
    }

    // Corner at the north end. Rot_x(+90°) puts local +Y→+Z
    // (continues north) and local +Z→-Y; Rot_z(180°) flips that
    // so +Z→+Y (the bend-arm goes up into the ceiling). Mirrored
    // runs spin an extra 180° around Y so the arm still meets
    // the wall instead of sticking into the corridor.
    let mut corner_rot = Quat::from_rotation_z(PI) * Quat::from_rotation_x(FRAC_PI_2);
    if mirror {
        corner_rot = Quat::from_rotation_y(PI) * corner_rot;
    }
    let corner_z = start_z + (STRAIGHT_SEGMENTS as f32) * PIPE_PIECE_LEN;
    ctx.prop_placement(
        PropPlacement::new(
            Prop::Pipe1Corner1,
            // +0.08 counters the PIPE_HANG drop and adds an 8 cm
            // nudge so the corner centreline matches the straights.
            Vec3::new(wall_x, y + 0.08, corner_z),
        )
        .rotated(corner_rot)
        .scaled(PIPE_SCALE)
        .no_collider(),
    );
}
