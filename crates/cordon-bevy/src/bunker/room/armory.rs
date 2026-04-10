//! Armory / supply cache zone: storage racks, crates, armchair.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::geometry::*;
use super::{Layout, Palette};

pub fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    l: &Layout,
) {
    // Storage racks along both walls — facing inward.
    glb(
        commands,
        asset_server,
        "models/storage/StorageRack_01.glb",
        Vec3::new(-l.hw + 0.6, 0.05, -2.2),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/storage/StorageRack_01.glb",
        Vec3::new(l.hw - 0.6, 0.05, -2.2),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Second pair further back.
    glb(
        commands,
        asset_server,
        "models/storage/StorageRack_01.glb",
        Vec3::new(-l.hw + 0.6, 0.05, -3.1),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    glb(
        commands,
        asset_server,
        "models/storage/StorageRack_01.glb",
        Vec3::new(l.hw - 0.6, 0.05, -3.1),
        Quat::from_rotation_y(-FRAC_PI_2),
    );

    // Crates on a pallet: back right corner (opposite the armchair).
    glb(
        commands,
        asset_server,
        "models/storage/EUR-Pallet.glb",
        Vec3::new(l.hw - 0.6, 0.1, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    glb(
        commands,
        asset_server,
        "models/storage/Crate_01.glb",
        Vec3::new(l.hw - 0.6, 0.25, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    glb(
        commands,
        asset_server,
        "models/storage/Crate_02.glb",
        Vec3::new(l.hw - 0.6, 0.55, l.back_z + 0.5),
        Quat::from_rotation_y(0.3),
    );
    // Loose box next to the pallet.
    glb(
        commands,
        asset_server,
        "models/storage/Box_01.glb",
        Vec3::new(l.hw - 1.2, 0.15, l.back_z + 0.4),
        Quat::from_rotation_y(0.2),
    );

    // Boxes stacked on the rack.
    glb(
        commands,
        asset_server,
        "models/storage/Box_02.glb",
        Vec3::new(-l.hw + 0.4, 0.7, -2.0),
        Quat::from_rotation_y(0.15),
    );

    // Armchair at the very back left, angled 45°.
    glb(
        commands,
        asset_server,
        "models/interior/Armchair1.glb",
        Vec3::new(-l.hw + 0.5, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2 / 2.0),
    );
    // Bag on the floor near the armchair.
    glb(
        commands,
        asset_server,
        "models/storage/Bag_01.glb",
        Vec3::new(-0.3, 0.1, l.back_z + 0.3),
        Quat::from_rotation_y(0.5),
    );

    // Back door (boarded up).
    spawn_box(
        commands,
        meshes,
        pal.wood.clone(),
        Vec3::new(0.0, 1.0, l.back_z + 0.05),
        Vec3::new(0.9, 2.0, 0.08),
    );
}
