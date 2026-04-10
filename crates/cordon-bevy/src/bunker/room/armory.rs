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
    prop(
        commands,
        asset_server,
        Prop::StorageRack01,
        Vec3::new(-l.hw + 0.6, 0.0, -2.2),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        commands,
        asset_server,
        Prop::StorageRack01,
        Vec3::new(l.hw - 0.6, 0.0, -2.2),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Second pair further back.
    prop(
        commands,
        asset_server,
        Prop::StorageRack01,
        Vec3::new(-l.hw + 0.6, 0.0, -3.1),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    prop(
        commands,
        asset_server,
        Prop::StorageRack01,
        Vec3::new(l.hw - 0.6, 0.0, -3.1),
        Quat::from_rotation_y(-FRAC_PI_2),
    );

    // Crates on a pallet: back right corner (opposite the armchair).
    // Pallet height 0.144m; Crate_01 height 0.513m; stack accordingly.
    prop(
        commands,
        asset_server,
        Prop::EURPallet,
        Vec3::new(l.hw - 0.6, 0.0, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    prop(
        commands,
        asset_server,
        Prop::Crate01,
        Vec3::new(l.hw - 0.6, 0.144, l.back_z + 0.5),
        Quat::IDENTITY,
    );
    prop(
        commands,
        asset_server,
        Prop::Crate02,
        Vec3::new(l.hw - 0.6, 0.657, l.back_z + 0.5),
        Quat::from_rotation_y(0.3),
    );
    // Loose box next to the pallet.
    prop(
        commands,
        asset_server,
        Prop::Box01,
        Vec3::new(l.hw - 1.2, 0.0, l.back_z + 0.4),
        Quat::from_rotation_y(0.2),
    );

    // Box on one of the rack shelves (eyeball — tune if it floats/clips).
    prop(
        commands,
        asset_server,
        Prop::Box02,
        Vec3::new(-l.hw + 0.4, 0.9, -2.0),
        Quat::from_rotation_y(0.15),
    );

    // Armchair at the very back left, angled 45°.
    prop(
        commands,
        asset_server,
        Prop::Armchair1,
        Vec3::new(-l.hw + 0.5, 0.0, l.back_z + 0.5),
        Quat::from_rotation_y(FRAC_PI_2 / 2.0),
    );
    // Bag on the floor near the armchair.
    prop(
        commands,
        asset_server,
        Prop::Bag01,
        Vec3::new(-0.3, 0.0, l.back_z + 0.3),
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
