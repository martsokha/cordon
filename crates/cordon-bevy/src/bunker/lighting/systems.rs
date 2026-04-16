use bevy::prelude::*;

use super::bundles::LightFixtureBundle;
use crate::bunker::resources::Layout;

pub fn spawn_lighting(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    l: &Layout,
) {
    let warm = Color::srgb(1.0, 0.82, 0.50);
    let cool = Color::srgb(0.85, 0.9, 1.0);
    let dim_cool = Color::srgb(0.8, 0.85, 0.95);
    let white = Color::srgb(0.95, 0.95, 1.0);
    let dim_warm = Color::srgb(1.0, 0.75, 0.45);
    let lamp_warm = Color::srgb(1.0, 0.70, 0.35);
    let screen_green = Color::srgb(0.4, 0.7, 0.4);

    let fixtures = [
        // Command post -- ceiling lamp pulled 1m back from the desk
        // so it illuminates the room, not just the table surface.
        LightFixtureBundle::ceiling(0.0, l.desk_z() - 1.0, l.h, 120000.0, warm, true),
        LightFixtureBundle::desk(Vec3::new(0.4, 0.95, l.desk_z() - 0.15), 8000.0, warm),
        LightFixtureBundle::screen(Vec3::new(0.0, 1.1, l.desk_z()), 6000.0, screen_green),
        // Entry
        LightFixtureBundle::ceiling(0.0, l.trade_z + 1.5, l.h, 50000.0, cool, false),
        // Armory + T-junction -- single light between them.
        LightFixtureBundle::ceiling(0.0, l.tj1_north - 0.5, l.h, 50000.0, dim_cool, false),
        // Kitchen
        LightFixtureBundle::ceiling(
            l.kitchen_x_center(),
            l.tj1_center(),
            l.h,
            45000.0,
            white,
            false,
        ),
        // Quarters
        LightFixtureBundle::ceiling(
            l.quarters_x_center(),
            l.tj1_center(),
            l.h,
            35000.0,
            dim_warm,
            false,
        ),
        LightFixtureBundle::standing(
            l.quarters_x_max() - 0.35,
            l.tj1_north - 0.4,
            18000.0,
            lamp_warm,
        ),
        // In-between hall: one fixture centred on the straight
        // segment so the passage between the two Ts isn't pitch
        // black.
        LightFixtureBundle::ceiling(
            0.0,
            (l.tj2_north + l.tj1_south) / 2.0,
            l.h,
            40000.0,
            dim_cool,
            false,
        ),
        // Infirmary: clinical white, slightly brighter than the
        // kitchen so the medical bay reads as well-lit.
        LightFixtureBundle::ceiling(
            l.infirmary_x_center(),
            l.tj2_center(),
            l.h,
            50000.0,
            white,
            false,
        ),
        // Workshop: cool industrial light.
        LightFixtureBundle::ceiling(
            l.workshop_x_center(),
            l.tj2_center(),
            l.h,
            45000.0,
            cool,
            false,
        ),
        // Back corridor end: dim fixture at the new back wall
        // so the corridor doesn't fade to black past T2.
        LightFixtureBundle::ceiling(0.0, l.back_z + 0.8, l.h, 35000.0, dim_cool, false),
    ];

    for fixture in &fixtures {
        fixture.spawn(commands, meshes, mats);
    }
}
