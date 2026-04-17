//! Command post zone: desk, laptop, chair, bookshelves, props.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::components::DoorButton;
use crate::bunker::geometry::*;
use crate::bunker::interaction::Interactable;
use crate::bunker::resources::{RadioPlacement, RoomCtx};

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    // Divider grate.
    let metal = ctx.pal.metal.clone();
    ctx.grate_bars(
        -ctx.l.hw,
        -ctx.l.hole_half,
        ctx.l.divider_z,
        ctx.l.h,
        0.12,
        &metal,
    );
    ctx.grate_bars(
        ctx.l.hole_half,
        ctx.l.hw,
        ctx.l.divider_z,
        ctx.l.h,
        0.12,
        &metal,
    );

    // Dinner table as the command desk. Table top at y = 1.037.
    const TABLE_TOP: f32 = 1.037;
    ctx.prop(Prop::WoodenDinnerTable, Vec3::new(0.0, 0.0, ctx.l.desk_z()));
    ctx.prop(Prop::WoodenChair, Vec3::new(0.0, 0.0, ctx.l.desk_z() - 0.5));
    ctx.prop(
        Prop::Mug,
        Vec3::new(-0.35, TABLE_TOP, ctx.l.desk_z() + 0.05),
    );
    // Radio placement: the radio module owns the spawn (like laptop).
    ctx.commands.insert_resource(RadioPlacement {
        pos: Vec3::new(-0.55, TABLE_TOP, ctx.l.desk_z() - 0.25),
        rot: Quat::from_rotation_y(PI),
    });

    // Door button — sits on the table surface. A flat cylinder
    // so it reads as a push button at a glance rather than a
    // mystery ball. Starts unlit; the visitor module's
    // `update_button_glow` flips `emissive` on this material when
    // someone is knocking.
    const BUTTON_RADIUS: f32 = 0.035;
    const BUTTON_HEIGHT: f32 = 0.015;
    let button_mesh = ctx.meshes.add(Cylinder::new(BUTTON_RADIUS, BUTTON_HEIGHT));
    let button_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.05, 0.05),
        perceptual_roughness: 0.4,
        metallic: 0.2,
        emissive: LinearRgba::BLACK,
        ..default()
    });
    ctx.commands.spawn((
        DoorButton,
        Interactable {
            key: "interact-door".into(),
            enabled: false,
        },
        Mesh3d(button_mesh),
        MeshMaterial3d(button_mat),
        // Centre sits half the cylinder height above the desk
        // surface so it rests flush instead of hovering.
        Transform::from_xyz(0.35, TABLE_TOP + BUTTON_HEIGHT / 2.0, ctx.l.desk_z()),
    ));
    // Bin between the table legs.
    ctx.prop_scaled(
        Prop::Bin,
        Vec3::new(-0.4, 0.0, ctx.l.desk_z()),
        Quat::IDENTITY,
        0.6,
    );

    // Two bookshelves per wall along the full command-post z-span.
    let shelf_north_z = ctx.l.trade_z - 0.978;
    let shelf_south_z = ctx.l.divider_z + 0.978;
    for z in [shelf_north_z, shelf_south_z] {
        ctx.prop_rot(
            Prop::Bookshelf,
            Vec3::new(-ctx.l.hw + 0.25, 0.0, z),
            Quat::from_rotation_y(FRAC_PI_2),
        );
        ctx.prop_rot(
            Prop::Bookshelf,
            Vec3::new(ctx.l.hw - 0.25, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }
    // Rug in front of the desk.
    ctx.prop(Prop::Rug, Vec3::new(0.0, 0.0, ctx.l.desk_z() - 0.3));
}
