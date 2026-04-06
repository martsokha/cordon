//! Laptop view: the Zone map with areas, bunker, and NPC dots.
//!
//! Renders area circles from game data. Hovering shows a localized
//! tooltip built from Fluent translation files.

use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy_fluent::prelude::*;
use bevy_lunex::prelude::*;
use cordon_core::primitive::environment::Environment;
use cordon_core::primitive::hazard::HazardType;
use cordon_core::primitive::tier::Tier;
use cordon_core::world::area::AreaDef;
use cordon_data::gamedata::GameDataResource;
use fluent_content::Content;

use crate::AppState;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((UiLunexPlugins, FluentPlugin));
        app.insert_resource(Locale::new("en-US".parse().expect("valid locale")));
        app.add_systems(Startup, (setup_camera, start_locale_load));
        app.add_systems(
            Update,
            build_localization
                .run_if(in_state(AppState::Loading))
                .run_if(resource_exists::<LocaleHandle>)
                .run_if(not(resource_exists::<GameLocalization>)),
        );
        app.add_systems(OnEnter(AppState::InGame), spawn_map);
        app.add_systems(
            Update,
            (move_npcs, update_tooltip).run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Resource)]
struct LocaleHandle(Handle<LoadedFolder>);

#[derive(Resource)]
pub struct GameLocalization(pub Localization);

#[derive(Component)]
struct Bunker;

#[derive(Component)]
struct AreaCircle;

#[derive(Component)]
struct AreaTooltipText(String);

#[derive(Component)]
struct NpcDot {
    direction: Vec2,
    speed: f32,
    home: Vec2,
    roam_radius: f32,
}

#[derive(Component)]
struct TooltipRoot;

const COLOR_BUNKER: Color = Color::srgb(1.0, 0.8, 0.2);
const COLOR_AREA: Color = Color::srgba(0.3, 0.6, 0.3, 0.15);
const COLOR_AREA_BORDER: Color = Color::srgba(0.3, 0.6, 0.3, 0.5);
const COLOR_AREA_HOVER: Color = Color::srgba(0.4, 0.8, 0.4, 0.25);
const COLOR_NPC: Color = Color::srgb(0.7, 0.7, 0.7);

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        UiSourceCamera::<0>,
        Transform::from_xyz(0.0, -100.0, 1000.0),
    ));
}

fn start_locale_load(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LocaleHandle(server.load_folder("locale")));
}

fn build_localization(
    handle: Res<LocaleHandle>,
    server: Res<AssetServer>,
    builder: LocalizationBuilder,
    mut commands: Commands,
) {
    if !server.is_loaded_with_dependencies(&handle.0) {
        return;
    }
    let l10n = builder.build(&handle.0);
    info!("Localization built: {:?}", l10n);
    commands.insert_resource(GameLocalization(l10n));
}

fn l10n_or(l10n: &Localization, key: &str, fallback: &str) -> String {
    l10n.content(key).unwrap_or_else(|| fallback.to_string())
}

fn tier_key(t: &Tier) -> &'static str {
    match t {
        Tier::VeryLow => "tier-verylow",
        Tier::Low => "tier-low",
        Tier::Medium => "tier-medium",
        Tier::High => "tier-high",
        Tier::VeryHigh => "tier-veryhigh",
    }
}

fn env_key(e: &Environment) -> &'static str {
    match e {
        Environment::Outdoor => "env-outdoor",
        Environment::Indoor => "env-indoor",
        Environment::Underground => "env-underground",
    }
}

fn hazard_key(h: &HazardType) -> &'static str {
    match h {
        HazardType::Chemical => "hazard-chemical",
        HazardType::Thermal => "hazard-thermal",
        HazardType::Electric => "hazard-electric",
        HazardType::Gravitational => "hazard-gravitational",
    }
}

fn build_tooltip_text(l10n: &Localization, area: &AreaDef) -> String {
    let name = l10n_or(l10n, &format!("area-{}", area.id.as_str()), area.id.as_str());
    let env = l10n_or(l10n, env_key(&area.environment), &format!("{:?}", area.environment));
    let creatures = l10n_or(l10n, tier_key(&area.danger.creatures), &format!("{:?}", area.danger.creatures));
    let radiation = l10n_or(l10n, tier_key(&area.danger.radiation), &format!("{:?}", area.danger.radiation));
    let hostility = l10n_or(l10n, tier_key(&area.danger.hostility), &format!("{:?}", area.danger.hostility));
    let hazards = if area.hazards.is_empty() {
        "None".to_string()
    } else {
        area.hazards.iter()
            .map(|h| l10n_or(l10n, hazard_key(h), &format!("{h:?}")))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let loot = l10n_or(l10n, tier_key(&area.loot_tier), &format!("{:?}", area.loot_tier));
    let faction = area.default_faction.as_ref()
        .map(|f| l10n_or(l10n, &format!("faction-{}", f.as_str()), f.as_str()))
        .unwrap_or_else(|| l10n_or(l10n, "faction-none", "Unclaimed"));

    format!(
        "{name}  ({env})\n\nCreatures: {creatures}   Radiation: {radiation}   Hostility: {hostility}\nHazards: {hazards}\nLoot: {loot}\nFaction: {faction}"
    )
}

fn spawn_map(
    game_data: Res<GameDataResource>,
    l10n: Option<Res<GameLocalization>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let data = &game_data.0;
    let empty_l10n = Localization::default();
    let l10n = l10n.as_ref().map(|r| &r.0).unwrap_or(&empty_l10n);

    commands.spawn((
        TooltipRoot,
        Text2d::new(""),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, 0.0, 100.0),
        Visibility::Hidden,
    ));

    for area in data.areas.values() {
        let x = area.location.x;
        let y = area.location.y;
        let radius = area.radius.value();
        let tooltip_text = build_tooltip_text(l10n, area);

        let area_entity = commands
            .spawn((
                AreaCircle,
                AreaTooltipText(tooltip_text),
                Dimension(Vec2::new(radius * 2.0, radius * 2.0)),
                Mesh2d(meshes.add(Circle::new(radius))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA))),
                Transform::from_xyz(x, y, 0.0),
            ))
            .id();

        commands.entity(area_entity).observe(
            move |_: On<Pointer<Over>>,
                  mut mats: ResMut<Assets<ColorMaterial>>,
                  mat_q: Query<&MeshMaterial2d<ColorMaterial>>,
                  text_q: Query<&AreaTooltipText>,
                  mut tip_q: Query<(&mut Text2d, &mut Visibility), With<TooltipRoot>>| {
                if let Ok(h) = mat_q.get(area_entity) {
                    if let Some(m) = mats.get_mut(&h.0) { m.color = COLOR_AREA_HOVER; }
                }
                if let Ok(t) = text_q.get(area_entity) {
                    for (mut txt, mut vis) in &mut tip_q {
                        txt.0.clone_from(&t.0);
                        *vis = Visibility::Visible;
                    }
                }
            },
        );

        commands.entity(area_entity).observe(
            move |_: On<Pointer<Out>>,
                  mut mats: ResMut<Assets<ColorMaterial>>,
                  mat_q: Query<&MeshMaterial2d<ColorMaterial>>,
                  mut tip_q: Query<&mut Visibility, With<TooltipRoot>>| {
                if let Ok(h) = mat_q.get(area_entity) {
                    if let Some(m) = mats.get_mut(&h.0) { m.color = COLOR_AREA; }
                }
                for mut vis in &mut tip_q { *vis = Visibility::Hidden; }
            },
        );

        commands.spawn((
            Mesh2d(meshes.add(Annulus::new(radius - 2.0, radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA_BORDER))),
            Transform::from_xyz(x, y, 0.1),
        ));

        for i in 0..2 {
            let angle = (i as f32) * 2.39996;
            commands.spawn((
                NpcDot {
                    direction: Vec2::new(angle.cos(), angle.sin()),
                    speed: 12.0 + (i as f32) * 4.0,
                    home: Vec2::new(x, y),
                    roam_radius: radius * 0.8,
                },
                Mesh2d(meshes.add(Circle::new(4.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_NPC))),
                Transform::from_xyz(
                    x + angle.cos() * radius * 0.3,
                    y + angle.sin() * radius * 0.3,
                    0.5,
                ),
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

fn update_tooltip(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut tip_q: Query<(&mut Transform, &Visibility), With<TooltipRoot>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((cam, cam_t)) = camera_q.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(world_pos) = cam.viewport_to_world_2d(cam_t, cursor) else { return };

    for (mut t, vis) in &mut tip_q {
        if *vis == Visibility::Visible {
            t.translation.x = world_pos.x + 20.0;
            t.translation.y = world_pos.y - 10.0;
        }
    }
}

fn move_npcs(time: Res<Time>, mut query: Query<(&mut NpcDot, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut npc, mut transform) in &mut query {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let new_pos = pos + npc.direction * npc.speed * dt;
        let dist = new_pos.distance(npc.home);
        if dist > npc.roam_radius {
            let to_home = (npc.home - new_pos).normalize_or_zero();
            npc.direction = (npc.direction * 0.3 + to_home * 0.7).normalize_or_zero();
        } else {
            let w = Vec2::new(
                (time.elapsed_secs() * 3.0 + transform.translation.x).sin() * 0.1,
                (time.elapsed_secs() * 2.7 + transform.translation.y).cos() * 0.1,
            );
            npc.direction = (npc.direction + w).normalize_or_zero();
        }
        transform.translation.x += npc.direction.x * npc.speed * dt;
        transform.translation.y += npc.direction.y * npc.speed * dt;
    }
}
