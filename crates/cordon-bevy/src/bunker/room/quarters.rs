//! Quarters (right side room): sofa, pillow, rug, personal items.

use std::f32::consts::{FRAC_PI_2, PI};

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
    let floor_half = Vec2::new(l.side_depth / 2.0, l.tj_len() / 2.0);
    spawn_floor_ceiling(
        commands,
        meshes,
        pal.concrete_dark.clone(),
        Vec3::new(l.quarters_x_center(), 0.0, l.tj_center()),
        floor_half,
        l.h,
    );

    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.quarters_x_max(), l.hh(), l.tj_center()),
        Quat::from_rotation_y(FRAC_PI_2),
        Vec2::new(l.tj_len() / 2.0, l.hh()),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.quarters_x_center(), l.hh(), l.tj_north),
        Quat::from_rotation_y(PI),
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(l.quarters_x_center(), l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.side_depth / 2.0, l.hh()),
    );

    // Wide sofa against the far wall.
    glb(
        commands,
        asset_server,
        "models/interior/WideSofa.glb",
        Vec3::new(l.quarters_x_max() - 0.5, 0.0, l.tj_center()),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Pillow.
    glb(
        commands,
        asset_server,
        "models/interior/Pillow.glb",
        Vec3::new(l.quarters_x_max() - 0.5, 0.4, l.tj_center() + 0.5),
        Quat::IDENTITY,
    );
    // Rug.
    glb(
        commands,
        asset_server,
        "models/interior/Rug.glb",
        Vec3::new(l.quarters_x_center(), 0.02, l.tj_center()),
        Quat::IDENTITY,
    );
    // Small bookshelf (personal books).
    glb(
        commands,
        asset_server,
        "models/interior/SingleBookshelf.glb",
        Vec3::new(l.quarters_x_max() - 0.3, 0.0, l.back_z + 0.3),
        Quat::from_rotation_y(-FRAC_PI_2),
    );
    // Suitcase — personal belongings.
    glb(
        commands,
        asset_server,
        "models/storage/Suitcase_01.glb",
        Vec3::new(l.quarters_x_center() - 0.3, 0.0, l.back_z + 0.3),
        Quat::from_rotation_y(0.4),
    );
    // Lamp next to the sofa.
    glb(
        commands,
        asset_server,
        "models/interior/Lamp1.glb",
        Vec3::new(l.quarters_x_max() - 0.3, 0.0, l.tj_center() - 0.8),
        Quat::IDENTITY,
    );
    // A bit of life.
    glb(
        commands,
        asset_server,
        "models/interior/PlantPot2.glb",
        Vec3::new(l.hw + 0.4, 0.0, l.back_z + 0.3),
        Quat::IDENTITY,
    );
    // Second pillow.
    glb(
        commands,
        asset_server,
        "models/interior/Pillow.glb",
        Vec3::new(l.quarters_x_max() - 0.5, 0.4, l.tj_center() - 0.3),
        Quat::from_rotation_y(0.5),
    );
    // Cactus in a pot — the one living thing down here.
    glb(
        commands,
        asset_server,
        "models/interior/PlantPot1.glb",
        Vec3::new(l.hw + 0.4, 0.0, l.tj_center() + 1.0),
        Quat::IDENTITY,
    );
    glb(
        commands,
        asset_server,
        "models/interior/Cactus.glb",
        Vec3::new(l.hw + 0.4, 0.25, l.tj_center() + 1.0),
        Quat::IDENTITY,
    );
}
