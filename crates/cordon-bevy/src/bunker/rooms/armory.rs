//! Armory / supply cache zone: storage racks, crates, armchair.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;

    // Two storage racks per wall, packed end-to-end along the armory
    // z-span. Rack width is 1.144 m. North margin from divider grate
    // is 0.20 m; gap between racks is 0.192 m; south rack extends
    // 0.15 m past `tj_north` into the T-junction's solid-wall strip.
    let rack_north_z = l.divider_z - 0.772;
    let rack_south_z = l.tj1_north - 0.15 + 0.572;
    for z in [rack_north_z, rack_south_z] {
        prop(
            ctx.commands,
            ctx.asset_server,
            Prop::StorageRack01,
            Vec3::new(-l.hw + 0.6, 0.0, z),
            Quat::from_rotation_y(FRAC_PI_2),
        );
        prop(
            ctx.commands,
            ctx.asset_server,
            Prop::StorageRack01,
            Vec3::new(l.hw - 0.6, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }

    // Back-corner props: pallet + stacked crates + loose box
    // against the corridor's deep south end (`back_z`), right
    // wall. Paired with the armchair on the opposite wall so the
    // corridor terminates in a lived-in scene rather than a bare
    // slab. Pallet height 0.144 m; Crate_01 height 0.513 m.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::EURPallet,
        Vec3::new(l.hw - 0.6, 0.0, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Crate01,
        Vec3::new(l.hw - 0.6, 0.144, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Crate02,
        Vec3::new(l.hw - 0.6, 0.657, l.back_z + 0.5),
        Quat::from_rotation_y(0.3),
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(l.hw - 1.2, 0.0, l.back_z + 0.4),
        Quat::from_rotation_y(0.2),
    );

    // Boxes and crates on the rack shelves. Each StorageRack_01 has
    // 3 shelves at approximately these heights:
    // Boxes sit at the rack's lateral center, slightly pulled toward
    // the corridor front so they read well from the aisle.
    const SHELF_BOTTOM: f32 = 0.65;
    const SHELF_MIDDLE: f32 = 1.30;
    const SHELF_TOP: f32 = 1.85;
    // Left wall, north rack.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box02,
        Vec3::new(-l.hw + 0.6, SHELF_MIDDLE, rack_north_z),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Crate01,
        Vec3::new(-l.hw + 0.6, SHELF_BOTTOM, rack_north_z),
        Quat::IDENTITY,
    );
    // Left wall, south rack.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(-l.hw + 0.6, SHELF_BOTTOM, rack_south_z),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box02,
        Vec3::new(-l.hw + 0.6, SHELF_TOP, rack_south_z),
        Quat::IDENTITY,
    );
    // Right wall, north rack.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(l.hw - 0.6, SHELF_MIDDLE, rack_north_z),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box02,
        Vec3::new(l.hw - 0.6, SHELF_TOP, rack_north_z),
        Quat::IDENTITY,
    );
    // Right wall, south rack.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Crate01,
        Vec3::new(l.hw - 0.6, SHELF_BOTTOM, rack_south_z),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(l.hw - 0.6, SHELF_MIDDLE, rack_south_z),
        Quat::IDENTITY,
    );

    // Armchair in the deep back corridor corner (left wall),
    // paired with the pallet+crates on the opposite wall so the
    // corridor terminates in a lived-in scene. Angled 45° toward
    // the corridor so someone sitting in it faces back up the
    // hall.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Armchair1,
        Vec3::new(-l.hw + 0.5, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2 / 2.0),
    );
    // Bag on the floor near the armchair.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Bag01,
        Vec3::new(-0.3, 0.0, l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );

}
