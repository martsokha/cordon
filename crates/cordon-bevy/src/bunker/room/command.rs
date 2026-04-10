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
    // Laptop with LaptopObject marker.
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
    // Bin.
    {
        let scene: Handle<Scene> = asset_server.load("models/interior/Bin.glb#Scene0");
        commands.spawn((
            SceneRoot(scene),
            Transform::from_xyz(-0.6, 0.0, l.desk_z() - 0.2).with_scale(Vec3::splat(0.6)),
        ));
    }
    // One bookshelf per wall — spaced to avoid clipping.
    glb(
        commands,
        asset_server,
        "models/interior/Bookshelf.glb",
        Vec3::new(-l.hw + 0.3, 0.0, 0.0),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/interior/Bookshelf.glb",
        Vec3::new(l.hw - 0.3, 0.0, 0.0),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Rug in front of the desk.
    glb(
        commands,
        asset_server,
        "models/interior/Rug.glb",
        Vec3::new(0.0, 0.02, l.desk_z() - 0.3),
        Quat::IDENTITY,
    );
    // Cactus in a pot.
    glb(
        commands,
        asset_server,
        "models/interior/PlantPot1.glb",
        Vec3::new(-l.hw + 0.3, 0.0, 0.0),
        Quat::IDENTITY,
    );
    glb(
        commands,
        asset_server,
        "models/interior/Cactus.glb",
        Vec3::new(-l.hw + 0.3, 0.25, 0.0),
        Quat::IDENTITY,
    );
    // Amp rack — comms equipment against the right wall.
    glb(
        commands,
        asset_server,
        "models/storage/AmpRack_01.glb",
        Vec3::new(l.hw - 0.3, 0.0, -0.8),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Filing cabinet behind the chair.
    glb(
        commands,
        asset_server,
        "models/storage/Cabinet_01.glb",
        Vec3::new(0.6, 0.0, l.desk_z() - 0.8),
        Quat::IDENTITY,
    );
    // Door button.
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
        Transform::from_xyz(0.28, 1.0, l.desk_z()),
    ));
}
