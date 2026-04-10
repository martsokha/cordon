//! Armory / supply cache zone: bookshelves, armchair, back door.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use super::geometry::*;
use super::{Layout, Palette};

pub fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    pal: &Palette,
    l: &Layout,
) {
    glb(commands, asset_server, "models/interior/Bookshelf.glb",
        Vec3::new(-l.hw + 0.3, 0.0, -2.2), Quat::from_rotation_y(FRAC_PI_2));
    glb(commands, asset_server, "models/interior/Bookshelf.glb",
        Vec3::new(l.hw - 0.3, 0.0, -2.2), Quat::from_rotation_y(-FRAC_PI_2));

    // Armchair at the very back, angled 45°.
    glb(commands, asset_server, "models/interior/Armchair1.glb",
        Vec3::new(-l.hw + 0.5, 0.0, l.back_z + 0.5), Quat::from_rotation_y(FRAC_PI_2 / 2.0));

    // Back door (boarded up).
    spawn_box(commands, meshes, pal.wood.clone(),
        Vec3::new(0.0, 1.0, l.back_z + 0.05), Vec3::new(0.9, 2.0, 0.08));
}
