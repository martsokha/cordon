//! Laptop view: the Zone map with areas, bunker, and NPC dots.
//!
//! Reads area definitions from [`GameDataResource`] and renders them
//! as circles. NPC dots roam within their assigned areas.

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;

use crate::AppState;

/// Bevy plugin for the laptop map view.
pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
        app.add_systems(OnEnter(AppState::InGame), spawn_map);
        app.add_systems(Update, move_npcs.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component)]
struct Bunker;

#[derive(Component)]
struct AreaMarker;

#[derive(Component)]
struct NpcDot {
    direction: Vec2,
    speed: f32,
    home: Vec2,
    roam_radius: f32,
}

const COLOR_BUNKER: Color = Color::srgb(1.0, 0.8, 0.2);
const COLOR_AREA: Color = Color::srgba(0.3, 0.6, 0.3, 0.15);
const COLOR_AREA_BORDER: Color = Color::srgba(0.3, 0.6, 0.3, 0.5);
const COLOR_NPC: Color = Color::srgb(0.7, 0.7, 0.7);

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::from_xyz(0.0, -100.0, 0.0)));
}

fn spawn_map(
    game_data: Res<GameDataResource>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let data = &game_data.0;

    for area in data.areas.values() {
        let x = area.location.x;
        let y = area.location.y;
        let radius = area.radius.value();

        commands.spawn((
            AreaMarker,
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA))),
            Transform::from_xyz(x, y, 0.0),
        ));

        commands.spawn((
            Mesh2d(meshes.add(Annulus::new(radius - 2.0, radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA_BORDER))),
            Transform::from_xyz(x, y, 0.1),
        ));

        for i in 0..2 {
            let angle = (i as f32) * 2.39996;
            let offset = radius * 0.3;
            let nx = x + angle.cos() * offset;
            let ny = y + angle.sin() * offset;

            commands.spawn((
                NpcDot {
                    direction: Vec2::new(angle.cos(), angle.sin()),
                    speed: 12.0 + (i as f32) * 4.0,
                    home: Vec2::new(x, y),
                    roam_radius: radius * 0.8,
                },
                Mesh2d(meshes.add(Circle::new(4.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_NPC))),
                Transform::from_xyz(nx, ny, 0.5),
            ));
        }
    }

    commands.spawn((
        Bunker,
        Mesh2d(meshes.add(Rectangle::new(16.0, 16.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_BUNKER))),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    info!("Laptop map: {} areas", data.areas.len());
}

fn move_npcs(time: Res<Time>, mut query: Query<(&mut NpcDot, &mut Transform)>) {
    let dt = time.delta_secs();

    for (mut npc, mut transform) in &mut query {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let new_pos = pos + npc.direction * npc.speed * dt;

        let dist_from_home = new_pos.distance(npc.home);
        if dist_from_home > npc.roam_radius {
            let to_home = (npc.home - new_pos).normalize_or_zero();
            npc.direction = (npc.direction * 0.3 + to_home * 0.7).normalize_or_zero();
        } else {
            let wobble = Vec2::new(
                (time.elapsed_secs() * 3.0 + transform.translation.x).sin() * 0.1,
                (time.elapsed_secs() * 2.7 + transform.translation.y).cos() * 0.1,
            );
            npc.direction = (npc.direction + wobble).normalize_or_zero();
        }

        transform.translation.x += npc.direction.x * npc.speed * dt;
        transform.translation.y += npc.direction.y * npc.speed * dt;
    }
}
