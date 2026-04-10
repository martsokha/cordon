//! Command post zone: desk, laptop, chair, bookshelves, props.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::geometry::*;
use super::{Layout, Palette};
use crate::bunker::{DoorButton, LaptopObject};

pub fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    pal: &Palette,
    l: &Layout,
) {
    // Divider grate.
    spawn_grate_bars(
        commands,
        meshes,
        pal.metal.clone(),
        -l.hw,
        -l.hole_half,
        l.divider_z,
        l.h,
        0.12,
    );
    spawn_grate_bars(
        commands,
        meshes,
        pal.metal.clone(),
        l.hole_half,
        l.hw,
        l.divider_z,
        l.h,
        0.12,
    );

    // Dinner table as the command desk.
    glb(
        commands,
        asset_server,
        "models/interior/WoodenDinnerTable.glb",
        Vec3::new(0.0, 0.0, l.desk_z()),
        Quat::IDENTITY,
    );
    // Chair.
    glb(
        commands,
        asset_server,
        "models/interior/WoodenChair.glb",
        Vec3::new(0.0, 0.0, l.desk_z() - 0.5),
        Quat::IDENTITY,
    );
    // Laptop.
    {
        let scene: Handle<Scene> = asset_server.load("models/interior/Laptop.glb#Scene0");
        commands.spawn((
            LaptopObject,
            SceneRoot(scene),
            Transform::from_xyz(0.0, 1.05, l.desk_z())
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        ));
    }
    // Mug.
    glb(
        commands,
        asset_server,
        "models/interior/Mug.glb",
        Vec3::new(-0.35, 1.05, l.desk_z() + 0.05),
        Quat::IDENTITY,
    );
    // Door button — raised above the table surface.
    commands.spawn((
        DoorButton,
        Mesh3d(meshes.add(Sphere::new(0.025))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.05, 0.05),
            perceptual_roughness: 0.4,
            metallic: 0.2,
            emissive: LinearRgba::BLACK,
            ..default()
        })),
        Transform::from_xyz(0.35, 1.07, l.desk_z()),
    ));
    // Bin between the table legs.
    {
        let scene: Handle<Scene> = asset_server.load("models/interior/Bin.glb#Scene0");
        commands.spawn((
            SceneRoot(scene),
            Transform::from_xyz(-0.4, 0.0, l.desk_z()).with_scale(Vec3::splat(0.6)),
        ));
    }
    // Two bookshelves per wall, packed tight against the trade grate.
    for z in [l.trade_z - 0.85, l.trade_z - 2.2] {
        // Right.
        glb(
            commands,
            asset_server,
            "models/interior/Bookshelf.glb",
            Vec3::new(-l.hw + 0.3, 0.0, z),
            Quat::from_rotation_y(FRAC_PI_2),
        );
        // Left.
        glb(
            commands,
            asset_server,
            "models/interior/Bookshelf.glb",
            Vec3::new(l.hw - 0.3, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }
    // Rug in front of the desk.
    glb(
        commands,
        asset_server,
        "models/interior/Rug.glb",
        Vec3::new(0.0, 0.02, l.desk_z() - 0.3),
        Quat::IDENTITY,
    );
    // Filing cabinet behind the chair.
    glb(
        commands,
        asset_server,
        "models/storage/Cabinet_01.glb",
        Vec3::new(0.6, 0.15, l.desk_z() - 0.8),
        Quat::IDENTITY,
    );
}
