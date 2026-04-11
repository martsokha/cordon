//! Entry checkpoint zone: stairs, trade grate, lockers, visitor side.

use bevy::prelude::*;

use super::RoomCtx;
use super::geometry::*;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    spawn_doorframe(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        0.0,
        l.front_z - 0.1,
        1.0,
        l.opening_h(),
    );
    spawn_stairs(
        ctx.commands,
        ctx.meshes,
        ctx.pal.concrete.clone(),
        l.front_z + 0.3,
        1.0,
        6,
    );

    // Trade grate: sides + bars below counter to block walking.
    spawn_grate_bars(
        ctx.commands,
        ctx.meshes,
        ctx.pal.metal.clone(),
        -l.hw,
        -l.hole_half,
        l.trade_z,
        l.h,
        0.1,
    );
    spawn_grate_bars(
        ctx.commands,
        ctx.meshes,
        ctx.pal.metal.clone(),
        l.hole_half,
        l.hw,
        l.trade_z,
        l.h,
        0.1,
    );
    spawn_box(
        ctx.commands,
        ctx.meshes,
        ctx.pal.wood.clone(),
        Vec3::new(0.0, 0.78, l.trade_z),
        Vec3::new(l.hole_half * 2.0 + 0.2, 0.04, 0.25),
    );
    spawn_grate_bars(
        ctx.commands,
        ctx.meshes,
        ctx.pal.metal.clone(),
        -l.hole_half,
        l.hole_half,
        l.trade_z,
        0.76,
        0.1,
    );
    // Invisible full-height collider across the center opening so
    // the player can't step over the short bars.
    ctx.commands.spawn((
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(l.hole_half * 2.0, l.h, 0.1),
        Transform::from_xyz(0.0, l.h / 2.0, l.trade_z),
    ));

    // Kitchen shelves on the visitor side of the trade grate,
    // facing away from the player — hides the player's legs.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::KitchenShelves2,
        Vec3::new(0.52, 0.0, l.trade_z + 0.1),
        Quat::IDENTITY,
    );
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::KitchenShelves2,
        Vec3::new(-0.52, 0.0, l.trade_z + 0.1),
        Quat::IDENTITY,
    );

    // Lockers along the left wall.
    for i in 0..5 {
        prop(
            ctx.commands,
            ctx.asset_server,
            Prop::Locker,
            Vec3::new(-l.hw + 0.3, 0.0, 2.2 + 0.5 * i as f32),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        );
    }
    // Bag on the floor near the lockers.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Bag02,
        Vec3::new(-l.hw + 0.8, 0.0, 4.0),
        Quat::from_rotation_y(0.8),
    );
    // Barrel in the corner.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Barrel03,
        Vec3::new(l.hw - 0.4, 0.0, 4.2),
        Quat::IDENTITY,
    );
    // Box near lockers.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Box01,
        Vec3::new(-l.hw + 0.8, 0.0, 3.2),
        Quat::from_rotation_y(0.3),
    );
    // Amp rack on the right wall (opposite lockers).
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::AmpRack01,
        Vec3::new(l.hw - 0.3, 0.0, 3.0),
        Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
    );
}
