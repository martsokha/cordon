//! Armory / supply cache zone: storage racks, crates, armchair.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    // Two storage racks per wall, packed end-to-end along the
    // armory z-span. Rack width is 1.144 m.
    let rack_north_z = ctx.l.divider_z - 0.772;
    let rack_south_z = ctx.l.tj1_north - 0.15 + 0.572;
    for z in [rack_north_z, rack_south_z] {
        ctx.prop_rot(
            Prop::StorageRack01,
            Vec3::new(-ctx.l.hw + 0.6, 0.0, z),
            Quat::from_rotation_y(FRAC_PI_2),
        );
        ctx.prop_rot(
            Prop::StorageRack01,
            Vec3::new(ctx.l.hw - 0.6, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }

    // Back-corner props: pallet + stacked crates + loose box
    // against the corridor's deep south end (right wall).
    ctx.prop(
        Prop::EURPallet,
        Vec3::new(ctx.l.hw - 0.6, 0.0, ctx.l.back_z + 0.5),
    );
    ctx.prop(
        Prop::Crate01,
        Vec3::new(ctx.l.hw - 0.6, 0.144, ctx.l.back_z + 0.5),
    );
    ctx.prop_rot(
        Prop::Crate02,
        Vec3::new(ctx.l.hw - 0.6, 0.657, ctx.l.back_z + 0.5),
        Quat::from_rotation_y(0.3),
    );
    ctx.prop_rot(
        Prop::Box01,
        Vec3::new(ctx.l.hw - 1.2, 0.0, ctx.l.back_z + 0.4),
        Quat::from_rotation_y(0.2),
    );

    // Boxes and crates on the rack shelves.
    const SHELF_BOTTOM: f32 = 0.65;
    const SHELF_MIDDLE: f32 = 1.30;
    const SHELF_TOP: f32 = 1.85;
    // Left wall.
    ctx.prop(
        Prop::Box02,
        Vec3::new(-ctx.l.hw + 0.6, SHELF_MIDDLE, rack_north_z),
    );
    ctx.prop(
        Prop::Crate01,
        Vec3::new(-ctx.l.hw + 0.6, SHELF_BOTTOM, rack_north_z),
    );
    ctx.prop(
        Prop::Box01,
        Vec3::new(-ctx.l.hw + 0.6, SHELF_BOTTOM, rack_south_z),
    );
    ctx.prop(
        Prop::Box02,
        Vec3::new(-ctx.l.hw + 0.6, SHELF_TOP, rack_south_z),
    );
    // Right wall.
    ctx.prop(
        Prop::Box01,
        Vec3::new(ctx.l.hw - 0.6, SHELF_MIDDLE, rack_north_z),
    );
    ctx.prop(
        Prop::Box02,
        Vec3::new(ctx.l.hw - 0.6, SHELF_TOP, rack_north_z),
    );
    ctx.prop(
        Prop::Crate01,
        Vec3::new(ctx.l.hw - 0.6, SHELF_BOTTOM, rack_south_z),
    );
    ctx.prop(
        Prop::Box01,
        Vec3::new(ctx.l.hw - 0.6, SHELF_MIDDLE, rack_south_z),
    );

    // Armchair + bag in the deep back corridor corner (left wall).
    ctx.prop_rot(
        Prop::Armchair1,
        Vec3::new(-ctx.l.hw + 0.5, 0.0, ctx.l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2 / 2.0),
    );
    ctx.prop_rot(
        Prop::Bag01,
        Vec3::new(-0.3, 0.0, ctx.l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );
}
