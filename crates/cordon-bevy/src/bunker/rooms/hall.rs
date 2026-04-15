//! The straight-hall segment between T1 and T2. Not a room —
//! just a ~2.5 m stretch of corridor (Z range
//! `[tj2_north, tj1_south]`) whose side walls are lined with
//! storage racks, two per wall.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

/// Half the centre-to-centre spacing between the two racks on a
/// wall. Rack half-width is 0.572 m; placing each centre at
/// `hall_cz ± 0.58` puts their outer edges at `hall_cz ± 1.152`,
/// just inside the hall's ±1.175 half-span. Empty wall on the
/// flanks is ~2 cm, matching the ~2 cm gap the racks leave
/// between themselves.
const RACK_OFFSET: f32 = 0.58;

/// Distance from the wall to the rack's lateral centre. Matches
/// the armory's 0.6 m inset so the racks sit at the same depth
/// into the room as their older siblings — deeper than strictly
/// needed to clear the wall, but reads consistent with existing
/// rack placements.
const WALL_INSET: f32 = 0.6;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    let hall_cz = (l.tj2_north + l.tj1_south) / 2.0;

    for side in [-1.0, 1.0] {
        let x = side * (l.hw - WALL_INSET);
        // Rotate each rack to face the hall interior. `FRAC_PI_2`
        // on the left wall (x < 0), `-FRAC_PI_2` on the right.
        let rot = Quat::from_rotation_y(-side * FRAC_PI_2);
        for z_off in [-RACK_OFFSET, RACK_OFFSET] {
            prop(
                ctx.commands,
                ctx.asset_server,
                Prop::StorageRack01,
                Vec3::new(x, 0.0, hall_cz + z_off),
                rot,
            );
        }
    }
}
