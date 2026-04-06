//! Laptop view: the Zone map with areas, bunker, and NPC dots.

use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy_fluent::prelude::*;
use bevy_lunex::prelude::*;
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
        app.insert_resource(TooltipContent::default());
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
            (move_npcs, follow_cursor, update_tooltip_ui).run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Resource)]
struct LocaleHandle(Handle<LoadedFolder>);

#[derive(Resource)]
pub struct GameLocalization(pub Localization);

#[derive(Resource, Default)]
struct TooltipContent {
    visible: bool,
    faction_icon: String,
    name: String,
    creatures: String,
    creatures_tier: Tier,
    radiation: String,
    radiation_tier: Tier,
    hazard_icon: String,
    loot: String,
    loot_tier: Tier,
}

#[derive(Component)]
struct Bunker;

#[derive(Component)]
struct AreaCircle;

#[derive(Component)]
struct AreaData(AreaTooltipInfo);

#[derive(Clone)]
struct AreaTooltipInfo {
    faction_icon: String,
    name: String,
    creatures: String,
    creatures_tier: Tier,
    radiation: String,
    radiation_tier: Tier,
    hazard_icon: String,
    loot: String,
    loot_tier: Tier,
}

#[derive(Component)]
struct NpcDot {
    direction: Vec2,
    speed: f32,
    home: Vec2,
    roam_radius: f32,
}

#[derive(Component)]
struct TooltipPanel;

#[derive(Component)]
struct TtHeader;

#[derive(Component)]
struct TtCreatures;

#[derive(Component)]
struct TtRadiation;

#[derive(Component)]
struct TtHazardIcon;

#[derive(Component)]
struct TtLoot;

const COLOR_BUNKER: Color = Color::srgb(1.0, 0.8, 0.2);
const COLOR_AREA: Color = Color::srgba(0.3, 0.6, 0.3, 0.15);
const COLOR_AREA_BORDER: Color = Color::srgba(0.3, 0.6, 0.3, 0.5);
const COLOR_AREA_HOVER: Color = Color::srgba(0.4, 0.8, 0.4, 0.25);
const COLOR_NPC: Color = Color::srgb(0.7, 0.7, 0.7);
const COLOR_LABEL: Color = Color::srgba(0.6, 0.6, 0.6, 1.0);

fn tier_color(t: &Tier) -> Color {
    match t {
        Tier::VeryLow => Color::srgb(0.5, 0.8, 0.5),
        Tier::Low => Color::srgb(0.7, 0.9, 0.4),
        Tier::Medium => Color::srgb(1.0, 0.85, 0.3),
        Tier::High => Color::srgb(1.0, 0.5, 0.2),
        Tier::VeryHigh => Color::srgb(1.0, 0.25, 0.25),
    }
}

fn hazard_icon(h: &HazardType) -> &'static str {
    match h {
        HazardType::Chemical => "X",
        HazardType::Thermal => "*",
        HazardType::Electric => "~",
        HazardType::Gravitational => "O",
    }
}

fn faction_icon_str(faction: Option<&str>) -> &'static str {
    match faction {
        Some("garrison") => "[G]",
        Some("syndicate") => "[S]",
        Some("order") => "[O]",
        Some("collective") => "[C]",
        Some("mercenaries") => "[M]",
        Some("institute") => "[I]",
        Some("devoted") => "[D]",
        Some("drifters") => "[d]",
        _ => "[ ]",
    }
}

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

fn build_area_info(l10n: &Localization, area: &AreaDef) -> AreaTooltipInfo {
    let hazard_icon_str = area
        .danger
        .hazard
        .as_ref()
        .map(|h| hazard_icon(&h.kind).to_string())
        .unwrap_or_default();

    AreaTooltipInfo {
        faction_icon: faction_icon_str(area.default_faction.as_ref().map(|f| f.as_str()))
            .to_string(),
        name: l10n_or(
            l10n,
            &format!("area-{}", area.id.as_str()),
            area.id.as_str(),
        ),
        creatures: l10n_or(
            l10n,
            tier_key(&area.danger.creatures),
            &format!("{:?}", area.danger.creatures),
        ),
        creatures_tier: area.danger.creatures,
        radiation: l10n_or(
            l10n,
            tier_key(&area.danger.radiation),
            &format!("{:?}", area.danger.radiation),
        ),
        radiation_tier: area.danger.radiation,
        hazard_icon: hazard_icon_str,
        loot: l10n_or(
            l10n,
            tier_key(&area.loot_tier),
            &format!("{:?}", area.loot_tier),
        ),
        loot_tier: area.loot_tier,
    }
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

    let hdr_font = TextFont {
        font_size: 14.0,
        ..default()
    };
    let lbl_font = TextFont {
        font_size: 11.0,
        ..default()
    };
    let val_font = TextFont {
        font_size: 12.0,
        ..default()
    };

    let mut panel = commands.spawn((
        TooltipPanel,
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(3.0),
            min_width: Val::Px(200.0),
            ..default()
        },
        Visibility::Hidden,
    ));
    panel
        .insert(BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.93)))
        .insert(GlobalZIndex(100));
    panel.with_children(|p| {
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                TtHeader,
                Text::new(""),
                hdr_font.clone(),
                TextColor(Color::WHITE),
            ));
            row.spawn((
                TtHazardIcon,
                Text::new(""),
                hdr_font.clone(),
                TextColor(Color::WHITE),
            ));
        });

        p.spawn(Node {
            height: Val::Px(4.0),
            ..default()
        });

        spawn_stat_row(p, "Creatures", TtCreatures, &lbl_font, &val_font);
        spawn_stat_row(p, "Radiation", TtRadiation, &lbl_font, &val_font);
        spawn_stat_row(p, "Loot", TtLoot, &lbl_font, &val_font);
    });

    for area in data.areas.values() {
        let x = area.location.x;
        let y = area.location.y;
        let radius = area.radius.value();
        let info = build_area_info(l10n, area);

        let area_entity = commands
            .spawn((
                AreaCircle,
                AreaData(info),
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
                  data_q: Query<&AreaData>,
                  mut tooltip: ResMut<TooltipContent>| {
                if let Ok(h) = mat_q.get(area_entity) {
                    if let Some(m) = mats.get_mut(&h.0) {
                        m.color = COLOR_AREA_HOVER;
                    }
                }
                if let Ok(d) = data_q.get(area_entity) {
                    let i = &d.0;
                    tooltip.visible = true;
                    tooltip.faction_icon.clone_from(&i.faction_icon);
                    tooltip.name.clone_from(&i.name);
                    tooltip.creatures.clone_from(&i.creatures);
                    tooltip.creatures_tier = i.creatures_tier;
                    tooltip.radiation.clone_from(&i.radiation);
                    tooltip.radiation_tier = i.radiation_tier;
                    tooltip.hazard_icon.clone_from(&i.hazard_icon);
                    tooltip.loot.clone_from(&i.loot);
                    tooltip.loot_tier = i.loot_tier;
                }
            },
        );

        commands.entity(area_entity).observe(
            move |_: On<Pointer<Out>>,
                  mut mats: ResMut<Assets<ColorMaterial>>,
                  mat_q: Query<&MeshMaterial2d<ColorMaterial>>,
                  mut tooltip: ResMut<TooltipContent>| {
                if let Ok(h) = mat_q.get(area_entity) {
                    if let Some(m) = mats.get_mut(&h.0) {
                        m.color = COLOR_AREA;
                    }
                }
                tooltip.visible = false;
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

fn spawn_stat_row(
    parent: &mut bevy::prelude::ChildSpawnerCommands,
    label: &str,
    marker: impl Component,
    lbl_font: &TextFont,
    val_font: &TextFont,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{label}:")),
                lbl_font.clone(),
                TextColor(COLOR_LABEL),
            ));
            row.spawn((
                marker,
                Text::new(""),
                val_font.clone(),
                TextColor(Color::WHITE),
            ));
        });
}

fn follow_cursor(
    tooltip: Res<TooltipContent>,
    windows: Query<&Window>,
    mut panel_q: Query<(&mut Node, &mut Visibility), With<TooltipPanel>>,
) {
    let cursor = windows
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
        .unwrap_or_default();
    for (mut node, mut vis) in &mut panel_q {
        if tooltip.visible {
            *vis = Visibility::Visible;
            node.left = Val::Px(cursor.x + 16.0);
            node.top = Val::Px(cursor.y + 16.0);
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_tooltip_ui(
    tooltip: Res<TooltipContent>,
    mut header_q: Query<&mut Text, (With<TtHeader>, Without<TtHazardIcon>)>,
    mut hazard_q: Query<&mut Text, (With<TtHazardIcon>, Without<TtHeader>)>,
    mut creatures_q: Query<
        (&mut Text, &mut TextColor),
        (With<TtCreatures>, Without<TtHeader>, Without<TtHazardIcon>),
    >,
    mut radiation_q: Query<
        (&mut Text, &mut TextColor),
        (
            With<TtRadiation>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtCreatures>,
        ),
    >,
    mut loot_q: Query<
        (&mut Text, &mut TextColor),
        (
            With<TtLoot>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtCreatures>,
            Without<TtRadiation>,
        ),
    >,
) {
    if !tooltip.is_changed() || !tooltip.visible {
        return;
    }

    let header = format!("{} {}", tooltip.faction_icon, tooltip.name);
    for mut t in &mut header_q {
        t.0 = header.clone();
    }
    for mut t in &mut hazard_q {
        t.0.clone_from(&tooltip.hazard_icon);
    }

    for (mut t, mut c) in &mut creatures_q {
        t.0.clone_from(&tooltip.creatures);
        c.0 = tier_color(&tooltip.creatures_tier);
    }
    for (mut t, mut c) in &mut radiation_q {
        t.0.clone_from(&tooltip.radiation);
        c.0 = tier_color(&tooltip.radiation_tier);
    }
    for (mut t, mut c) in &mut loot_q {
        t.0.clone_from(&tooltip.loot);
        c.0 = tier_color(&tooltip.loot_tier);
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
