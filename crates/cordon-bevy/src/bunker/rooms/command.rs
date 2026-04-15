//! Command post zone: desk, laptop, chair, bookshelves, props.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;

use crate::bunker::components::DoorButton;
use crate::bunker::geometry::*;
use crate::bunker::interaction::Interactable;
use crate::bunker::resources::RoomCtx;

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let l = ctx.l;
    // Divider grate.
    spawn_grate_bars(
        ctx.commands,
        ctx.meshes,
        ctx.pal.metal.clone(),
        -l.hw,
        -l.hole_half,
        l.divider_z,
        l.h,
        0.12,
    );
    spawn_grate_bars(
        ctx.commands,
        ctx.meshes,
        ctx.pal.metal.clone(),
        l.hole_half,
        l.hw,
        l.divider_z,
        l.h,
        0.12,
    );

    // Dinner table as the command desk. Table top is at y = 1.037.
    const TABLE_TOP: f32 = 1.037;
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::WoodenDinnerTable,
        Vec3::new(0.0, 0.0, l.desk_z()),
        Quat::IDENTITY,
    );
    // Chair.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::WoodenChair,
        Vec3::new(0.0, 0.0, l.desk_z() - 0.5),
        Quat::IDENTITY,
    );
    // Mug.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Mug,
        Vec3::new(-0.35, TABLE_TOP, l.desk_z() + 0.05),
        Quat::IDENTITY,
    );
    // Radio on the desk, left of the laptop and back from the
    // player — faces the player so the dials read.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Radio,
        Vec3::new(-0.55, TABLE_TOP, l.desk_z() - 0.25),
        Quat::from_rotation_y(PI),
    );
    // Door button — sits on the table surface. Starts disabled;
    // visitor module enables it when someone is knocking.
    let button_mesh = ctx.meshes.add(Sphere::new(0.025));
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
            prompt: "[E] Open Door",
            enabled: false,
        },
        Mesh3d(button_mesh),
        MeshMaterial3d(button_mat),
        Transform::from_xyz(0.35, TABLE_TOP + 0.03, l.desk_z()),
    ));
    // Bin between the table legs (scaled down; kept as raw spawn).
    {
        let scene: Handle<Scene> = ctx.asset_server.load("models/interior/Bin.glb#Scene0");
        ctx.commands.spawn((
            SceneRoot(scene),
            Transform::from_xyz(-0.4, 0.0, l.desk_z()).with_scale(Vec3::splat(0.6)),
        ));
    }
    // Two bookshelves per wall along the full command-post z-span.
    // Bookshelf is 1.656 m wide; with the 3.75 m command post we fit
    // two end-to-end with ~0.14 m gap and ~0.15 m margins at each end.
    // Shelves are laterally off the corridor centerline so they don't
    // interfere with the desk ensemble at x = 0.
    let shelf_north_z = l.trade_z - 0.978;
    let shelf_south_z = l.divider_z + 0.978;
    for z in [shelf_north_z, shelf_south_z] {
        prop(
            ctx.commands,
            ctx.asset_server,
            Prop::Bookshelf,
            Vec3::new(-l.hw + 0.25, 0.0, z),
            Quat::from_rotation_y(FRAC_PI_2),
        );
        prop(
            ctx.commands,
            ctx.asset_server,
            Prop::Bookshelf,
            Vec3::new(l.hw - 0.25, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }
    // Rug in front of the desk.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Rug,
        Vec3::new(0.0, 0.0, l.desk_z() - 0.3),
        Quat::IDENTITY,
    );
    // Filing cabinet behind the chair.
    prop(
        ctx.commands,
        ctx.asset_server,
        Prop::Cabinet01,
        Vec3::new(0.6, 0.0, l.desk_z() - 0.8),
        Quat::IDENTITY,
    );
}
