//! Laptop view: the Zone map with areas, bunker, and NPC dots.

use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy_fluent::prelude::*;
use bevy_lunex::prelude::*;
use cordon_core::entity::faction::RankScheme;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::primitive::hazard::HazardType;
use cordon_core::primitive::tier::Tier;
use cordon_core::primitive::uid::Uid;
use cordon_core::world::area::AreaDef;
use cordon_data::gamedata::GameDataResource;
use fluent_content::Content;

use crate::AppState;
use crate::world::SimWorld;

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
        app.add_systems(
            OnEnter(AppState::InGame),
            spawn_map.after(crate::world::init_world),
        );
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
enum TooltipContent {
    #[default]
    Hidden,
    Area {
        faction_icon: String,
        name: String,
        creatures: String,
        creatures_tier: Tier,
        radiation: String,
        radiation_tier: Tier,
        hazard_icon: String,
        loot: String,
        loot_tier: Tier,
    },
    Npc {
        faction_icon: String,
        name: String,
        faction: String,
        rank: String,
    },
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
    uid: Uid,
    direction: Vec2,
    speed: f32,
    home: Vec2,
    roam_radius: f32,
}

#[derive(Component, Clone)]
struct NpcDotInfo {
    faction_icon: String,
    name: String,
    faction: String,
    rank: String,
}

#[derive(Component)]
struct TooltipPanel;

#[derive(Component)]
struct TtHeader;

#[derive(Component)]
struct TtRow1Label;
#[derive(Component)]
struct TtRow1Value;

#[derive(Component)]
struct TtRow2Label;
#[derive(Component)]
struct TtRow2Value;

#[derive(Component)]
struct TtHazardIcon;

#[derive(Component)]
struct TtRow3Label;
#[derive(Component)]
struct TtRow3Value;

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

fn rank_scheme_key(scheme: &RankScheme) -> &'static str {
    match scheme {
        RankScheme::Military => "military",
        RankScheme::Loose => "loose",
        RankScheme::Religious => "religious",
        RankScheme::Academic => "academic",
    }
}

fn resolve_npc_name(l10n: &Localization, name: &NpcName) -> String {
    let first = l10n_or(l10n, &name.first, &name.first);
    match (&name.format, &name.second) {
        (NameFormat::Alias, _) => first,
        (NameFormat::FirstSurname, Some(second)) => {
            let second = l10n_or(l10n, second, second);
            format!("{first} {second}")
        }
        (NameFormat::FirstAlias, Some(second)) => {
            let second = l10n_or(l10n, second, second);
            format!("{first} \"{second}\"")
        }
        _ => first,
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
    sim_world: Res<SimWorld>,
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

        spawn_stat_row(
            p,
            "Creatures",
            TtRow1Label,
            TtRow1Value,
            &lbl_font,
            &val_font,
        );
        spawn_stat_row(
            p,
            "Radiation",
            TtRow2Label,
            TtRow2Value,
            &lbl_font,
            &val_font,
        );
        spawn_stat_row(p, "Loot", TtRow3Label, TtRow3Value, &lbl_font, &val_font);
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
                    *tooltip = TooltipContent::Area {
                        faction_icon: i.faction_icon.clone(),
                        name: i.name.clone(),
                        creatures: i.creatures.clone(),
                        creatures_tier: i.creatures_tier,
                        radiation: i.radiation.clone(),
                        radiation_tier: i.radiation_tier,
                        hazard_icon: i.hazard_icon.clone(),
                        loot: i.loot.clone(),
                        loot_tier: i.loot_tier,
                    };
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
                *tooltip = TooltipContent::Hidden;
            },
        );

        commands.spawn((
            Mesh2d(meshes.add(Annulus::new(radius - 2.0, radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA_BORDER))),
            Transform::from_xyz(x, y, 0.1),
        ));
    }

    commands.spawn((
        Bunker,
        Mesh2d(meshes.add(Rectangle::new(16.0, 16.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_BUNKER))),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    let bunker_pos = Vec2::ZERO;
    let bunker_radius = 40.0;
    for (i, (uid, npc)) in sim_world.0.npcs.iter().enumerate() {
        let angle = (i as f32) * 2.39996;
        let offset = bunker_radius * 0.3 + (i as f32 % 5.0) * 8.0;
        let dot_size = 4.0;

        let faction_icon = faction_icon_str(Some(npc.faction.as_str())).to_string();
        let faction_name = l10n_or(
            l10n,
            &format!("faction-{}", npc.faction.as_str()),
            npc.faction.as_str(),
        );
        let name_display = resolve_npc_name(l10n, &npc.name);
        let rank_title = data
            .faction(&npc.faction)
            .map(|fdef| {
                let key = format!("rank-{}-{}", rank_scheme_key(&fdef.rank_scheme), npc.rank());
                l10n_or(l10n, &key, &key)
            })
            .unwrap_or_else(|| format!("Rank {}", npc.rank()));

        let npc_entity = commands
            .spawn((
                NpcDot {
                    uid: *uid,
                    direction: Vec2::new(angle.cos(), angle.sin()),
                    speed: 10.0 + (i as f32 % 3.0) * 4.0,
                    home: bunker_pos,
                    roam_radius: bunker_radius,
                },
                NpcDotInfo {
                    faction_icon: faction_icon.clone(),
                    name: name_display,
                    faction: faction_name,
                    rank: rank_title,
                },
                Dimension(Vec2::splat(dot_size * 2.0)),
                Mesh2d(meshes.add(Circle::new(dot_size))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_NPC))),
                Transform::from_xyz(
                    bunker_pos.x + angle.cos() * offset,
                    bunker_pos.y + angle.sin() * offset,
                    0.5,
                ),
            ))
            .id();

        commands.entity(npc_entity).observe(
            move |_: On<Pointer<Over>>,
                  data_q: Query<&NpcDotInfo>,
                  mut tooltip: ResMut<TooltipContent>| {
                if let Ok(info) = data_q.get(npc_entity) {
                    *tooltip = TooltipContent::Npc {
                        faction_icon: info.faction_icon.clone(),
                        name: info.name.clone(),
                        faction: info.faction.clone(),
                        rank: info.rank.clone(),
                    };
                }
            },
        );

        commands.entity(npc_entity).observe(
            move |_: On<Pointer<Out>>, mut tooltip: ResMut<TooltipContent>| {
                *tooltip = TooltipContent::Hidden;
            },
        );
    }

    info!(
        "Laptop map: {} areas, {} npcs",
        data.areas.len(),
        sim_world.0.npcs.len()
    );
}

fn spawn_stat_row(
    parent: &mut bevy::prelude::ChildSpawnerCommands,
    label: &str,
    lbl_marker: impl Component,
    val_marker: impl Component,
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
                lbl_marker,
                Text::new(format!("{label}:")),
                lbl_font.clone(),
                TextColor(COLOR_LABEL),
            ));
            row.spawn((
                val_marker,
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
    let visible = !matches!(*tooltip, TooltipContent::Hidden);
    for (mut node, mut vis) in &mut panel_q {
        if visible {
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
    mut r1_lbl: Query<&mut Text, (With<TtRow1Label>, Without<TtHeader>, Without<TtHazardIcon>)>,
    mut r1_val: Query<
        (&mut Text, &mut TextColor),
        (
            With<TtRow1Value>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtRow1Label>,
        ),
    >,
    mut r2_lbl: Query<
        &mut Text,
        (
            With<TtRow2Label>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtRow1Label>,
            Without<TtRow1Value>,
        ),
    >,
    mut r2_val: Query<
        (&mut Text, &mut TextColor),
        (
            With<TtRow2Value>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtRow1Label>,
            Without<TtRow1Value>,
            Without<TtRow2Label>,
        ),
    >,
    mut r3_lbl: Query<
        &mut Text,
        (
            With<TtRow3Label>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtRow1Label>,
            Without<TtRow1Value>,
            Without<TtRow2Label>,
            Without<TtRow2Value>,
        ),
    >,
    mut r3_val: Query<
        (&mut Text, &mut TextColor),
        (
            With<TtRow3Value>,
            Without<TtHeader>,
            Without<TtHazardIcon>,
            Without<TtRow1Label>,
            Without<TtRow1Value>,
            Without<TtRow2Label>,
            Without<TtRow2Value>,
            Without<TtRow3Label>,
        ),
    >,
) {
    if !tooltip.is_changed() || matches!(*tooltip, TooltipContent::Hidden) {
        return;
    }

    match &*tooltip {
        TooltipContent::Hidden => {}
        TooltipContent::Area {
            faction_icon,
            name,
            creatures,
            creatures_tier,
            radiation,
            radiation_tier,
            hazard_icon,
            loot,
            loot_tier,
        } => {
            for mut t in &mut header_q {
                t.0 = format!("{faction_icon} {name}");
            }
            for mut t in &mut hazard_q {
                t.0.clone_from(hazard_icon);
            }
            for mut t in &mut r1_lbl {
                t.0 = "Creatures:".into();
            }
            for (mut t, mut c) in &mut r1_val {
                t.0.clone_from(creatures);
                c.0 = tier_color(creatures_tier);
            }
            for mut t in &mut r2_lbl {
                t.0 = "Radiation:".into();
            }
            for (mut t, mut c) in &mut r2_val {
                t.0.clone_from(radiation);
                c.0 = tier_color(radiation_tier);
            }
            for mut t in &mut r3_lbl {
                t.0 = "Loot:".into();
            }
            for (mut t, mut c) in &mut r3_val {
                t.0.clone_from(loot);
                c.0 = tier_color(loot_tier);
            }
        }
        TooltipContent::Npc {
            faction_icon,
            name,
            faction,
            rank,
        } => {
            for mut t in &mut header_q {
                t.0 = format!("{faction_icon} {name}");
            }
            for mut t in &mut hazard_q {
                t.0.clear();
            }
            for mut t in &mut r1_lbl {
                t.0 = "Faction:".into();
            }
            for (mut t, mut c) in &mut r1_val {
                t.0.clone_from(faction);
                c.0 = Color::WHITE;
            }
            for mut t in &mut r2_lbl {
                t.0 = "Rank:".into();
            }
            for (mut t, mut c) in &mut r2_val {
                t.0.clone_from(rank);
                c.0 = Color::WHITE;
            }
            for mut t in &mut r3_lbl {
                t.0.clear();
            }
            for (mut t, _) in &mut r3_val {
                t.0.clear();
            }
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
