//! Entry checkpoint zone: stairs, trade grate, lockers, visitor side.

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use crate::bunker::geometry::*;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    // Entry door: Door2 scaled 1.44 to match the 2.10 m opening.
    const DOOR_SCALE: f32 = 1.44;
    ctx.prop_scaled(
        Prop::Door2,
        Vec3::new(0.0, 0.0, ctx.l.front_z - 0.1),
        Quat::IDENTITY,
        DOOR_SCALE,
    );
    // Matching door on the opposite end of the corridor.
    ctx.prop_scaled(
        Prop::Door2,
        Vec3::new(0.0, 0.0, ctx.l.back_z + 0.1),
        Quat::from_rotation_y(PI),
        DOOR_SCALE,
    );

    let concrete = ctx.pal.concrete.clone();
    let metal = ctx.pal.metal.clone();
    let wood = ctx.pal.wood.clone();
    ctx.stairs(ctx.l.front_z + 0.3, 1.0, 6, &concrete);

    // Trade grate: sides + bars below counter to block walking.
    ctx.grate_bars(
        -ctx.l.hw,
        -ctx.l.hole_half,
        ctx.l.trade_z,
        ctx.l.h,
        0.1,
        &metal,
    );
    ctx.grate_bars(
        ctx.l.hole_half,
        ctx.l.hw,
        ctx.l.trade_z,
        ctx.l.h,
        0.1,
        &metal,
    );
    ctx.decor_box(
        Vec3::new(0.0, 0.78, ctx.l.trade_z),
        Vec3::new(ctx.l.hole_half * 2.0 + 0.2, 0.04, 0.25),
        &wood,
    );
    ctx.grate_bars(
        -ctx.l.hole_half,
        ctx.l.hole_half,
        ctx.l.trade_z,
        0.76,
        0.1,
        &metal,
    );
    // Invisible full-height collider across the center opening.
    ctx.commands.spawn((
        RigidBody::Static,
        Collider::cuboid(ctx.l.hole_half * 2.0, ctx.l.h, 0.1),
        Transform::from_xyz(0.0, ctx.l.h / 2.0, ctx.l.trade_z),
    ));

    // Kitchen shelves on the visitor side of the trade grate,
    // facing away from the player — hides the player's legs.
    ctx.prop(
        Prop::KitchenShelves2,
        Vec3::new(0.52, 0.0, ctx.l.trade_z + 0.1),
    );
    ctx.prop(
        Prop::KitchenShelves2,
        Vec3::new(-0.52, 0.0, ctx.l.trade_z + 0.1),
    );

    // Lockers along the left wall, starting 0.7 m north of the grate.
    for i in 0..5 {
        ctx.prop_rot(
            Prop::Locker,
            Vec3::new(-ctx.l.hw + 0.3, 0.0, ctx.l.trade_z + 0.7 + 0.5 * i as f32),
            Quat::from_rotation_y(FRAC_PI_2),
        );
    }
    ctx.prop_rot(
        Prop::Bag02,
        Vec3::new(-ctx.l.hw + 0.8, 0.0, ctx.l.trade_z + 2.5),
        Quat::from_rotation_y(0.8),
    );
    ctx.prop(
        Prop::Barrel03,
        Vec3::new(ctx.l.hw - 0.4, 0.0, ctx.l.trade_z + 2.7),
    );
    ctx.prop_rot(
        Prop::Box01,
        Vec3::new(-ctx.l.hw + 0.8, 0.0, ctx.l.trade_z + 1.7),
        Quat::from_rotation_y(0.3),
    );
    // Amp rack on the right wall (opposite lockers).
    ctx.prop_rot(
        Prop::AmpRack01,
        Vec3::new(ctx.l.hw - 0.3, 0.0, ctx.l.trade_z + 1.5),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
}
