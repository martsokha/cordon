//! Ceiling pipes. Decorative industrial detail made of atomic-
//! pack pipe pieces composited into a run under the corridor
//! ceiling.
//!
//! The pre-composed long segments from the pack ship with weird
//! off-origin AABBs, so instead we chain `Pipe1Long` (~3 m)
//! pieces end-to-end ourselves. Each piece is rotated -90° around
//! X so its native vertical axis aligns with world Z (down the
//! corridor).

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

/// Uniform scale on every pipe piece. At 1.0 the body is ~0.88 m
/// wide; 0.25 brings it to ~22 cm (utility conduit). Uniform
/// scaling means 3 m straight pieces shrink to 0.75 m — that's
/// fine, the run is still continuous.
const PIPE_SCALE: f32 = 0.25;

/// Length of one `Pipe1Long` piece after [`PIPE_SCALE`].
const PIPE_PIECE_LEN: f32 = 3.0 * PIPE_SCALE;

/// Number of straight segments before the pipe bends into the
/// wall via a corner piece.
const STRAIGHT_SEGMENTS: u32 = 6;

/// How far under the ceiling the pipe centreline hangs.
const PIPE_HANG: f32 = 0.24;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;

    // Start the run touching the back wall itself. The AABB-
    // based feet-offset leaves the mesh's south edge right on
    // `back_z`, so the run visually terminates at the wall.
    let start_z = l.back_z;

    // `Pipe1Long`'s long axis is native Y. Rotating +90° around X
    // sends local +Y to world +Z, so the piece lies horizontal
    // and extends north up the corridor.
    let straight_rot = Quat::from_rotation_x(FRAC_PI_2);

    // Halfway between the left wall (-hw = -2.05) and the
    // corridor centre; pipe sits off-axis without clipping the
    // wall or stealing the middle of the corridor.
    let wall_x = -1.3;
    let y = l.h - PIPE_HANG;

    for i in 0..STRAIGHT_SEGMENTS {
        let z = start_z + (i as f32) * PIPE_PIECE_LEN;
        prop_scaled(
            ctx.commands,
            ctx.asset_server,
            Prop::Pipe1Long,
            Vec3::new(wall_x, y, z),
            straight_rot,
            PIPE_SCALE,
        );
    }

    // Corner at the north end of the run. Its two arms should
    // point along +Z (to continue the run) and +Y (up to the
    // ceiling).
    //
    // `Pipe1Corner1` local axes: long-arm along +Y, bend-arm
    // along +Z, pipe body thickness along +X.
    // Rot_x(+90°) sends local +Y→+Z (good, arm continues
    //   north) and local +Z→-Y (arm goes *down*, wrong).
    // Rot_z(180°) flips Y so -Y becomes +Y.
    // Net: local +Y→+Z (continues north), local +Z→+Y (up).
    let corner_rot = Quat::from_rotation_z(PI) * Quat::from_rotation_x(FRAC_PI_2);
    let corner_z = start_z + (STRAIGHT_SEGMENTS as f32) * PIPE_PIECE_LEN;
    prop_scaled(
        ctx.commands,
        ctx.asset_server,
        Prop::Pipe1Corner1,
        // Corner rides 4 cm higher than the straight run: 3 cm
        // to counter the PIPE_HANG drop that moved the straights
        // down, plus the 1 cm nudge you asked for on top.
        Vec3::new(wall_x, y + 0.08, corner_z),
        corner_rot,
        PIPE_SCALE,
    );
}
