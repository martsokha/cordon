//! Laptop view: the Zone map with areas, bunker, and NPC dots.

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use bevy_lunex::prelude::*;
use cordon_core::entity::faction::{Faction, RankScheme};
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::hazard::HazardType;
use cordon_core::primitive::id::Id;
use cordon_core::primitive::tier::Tier;
use cordon_core::primitive::uid::Uid;
use cordon_core::world::area::AreaDef;
use cordon_data::gamedata::GameDataResource;

use crate::AppState;
use crate::ai::behavior::{Action, Intent};
use crate::locale::{GameLocalization, l10n_or};
use crate::world::SimWorld;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins);
        app.insert_resource(TooltipContent::default());
        app.insert_resource(SelectedNpc::default());
        app.add_systems(Startup, setup_camera);
        app.add_systems(
            OnEnter(AppState::InGame),
            spawn_map.after(crate::world::init_world),
        );
        app.add_systems(
            Update,
            (
                follow_cursor,
                update_tooltip_ui,
                handle_npc_click,
                update_npc_selection,
                deselect_on_escape,
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

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
        status: String,
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
    uid: Uid<Npc>,
}

#[derive(Component, Clone)]
struct NpcDotInfo {
    faction_icon: String,
    name: String,
    faction: String,
    rank: String,
}

#[derive(Component, Clone)]
struct NpcFaction(Id<Faction>);

#[derive(Resource, Default)]
struct SelectedNpc(Option<Uid<Npc>>);

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
const COLOR_NPC_SELECTED: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NPC_SQUAD: Color = Color::srgb(0.9, 0.75, 0.3);
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

fn format_npc_status(action: &Action, intent: &Intent) -> String {
    let doing = match action {
        Action::Idle { .. } => "Idle",
        Action::Walk { .. } => "Walking",
        Action::Follow { .. } => "Following",
        Action::Trade => "Trading",
        Action::Flee { .. } => "Fleeing",
    };
    let goal = match intent {
        Intent::Visit => "visiting",
        Intent::Scavenge(_) => "scavenging",
        Intent::Patrol(_) => "patrolling",
        Intent::Quest(_) => "on a quest",
        Intent::Escort(_) => "escorting",
        Intent::Recruit => "looking for work",
        Intent::Leave => "leaving",
    };
    format!("{doing} ({goal})")
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
    let spawn_radius = 80.0;
    let dot_size = 4.0;
    let hit_size = 20.0;
    for (i, (uid, npc)) in sim_world.0.npcs.iter().enumerate() {
        let angle = (i as f32) * 2.39996;
        let spawn_pos = Vec2::new(
            bunker_pos.x + angle.cos() * spawn_radius,
            bunker_pos.y + angle.sin() * spawn_radius,
        );

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

        let idle_pos = Vec2::new(
            bunker_pos.x + angle.cos() * 25.0,
            bunker_pos.y + angle.sin() * 25.0,
        );

        let npc_entity = commands
            .spawn((
                NpcDot { uid: *uid },
                Action::Walk {
                    target: idle_pos,
                    speed: 15.0,
                },
                Intent::Visit,
                NpcDotInfo {
                    faction_icon: faction_icon.clone(),
                    name: name_display,
                    faction: faction_name,
                    rank: rank_title,
                },
                NpcFaction(npc.faction.clone()),
                Dimension(Vec2::splat(hit_size)),
                Mesh2d(meshes.add(Circle::new(dot_size))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_NPC))),
                Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.5),
            ))
            .id();

        commands.entity(npc_entity).observe(
            move |_: On<Pointer<Over>>,
                  data_q: Query<(&NpcDotInfo, &Action, &Intent)>,
                  mut tooltip: ResMut<TooltipContent>| {
                if let Ok((info, action, intent)) = data_q.get(npc_entity) {
                    *tooltip = TooltipContent::Npc {
                        faction_icon: info.faction_icon.clone(),
                        name: info.name.clone(),
                        faction: info.faction.clone(),
                        rank: info.rank.clone(),
                        status: format_npc_status(action, intent),
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
            status,
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
                t.0 = "Status:".into();
            }
            for (mut t, mut c) in &mut r3_val {
                t.0.clone_from(status);
                c.0 = COLOR_LABEL;
            }
        }
    }
}

fn handle_npc_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    dots: Query<(&NpcDot, &Transform)>,
    mut selected: ResMut<SelectedNpc>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor_screen) = windows.single().ok().and_then(|w| w.cursor_position()) else {
        return;
    };
    let Some((camera, cam_transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(cursor_world) = camera.viewport_to_world_2d(cam_transform, cursor_screen) else {
        return;
    };

    let hit_radius = 20.0;
    let mut closest: Option<(Uid<Npc>, f32)> = None;
    for (dot, transform) in &dots {
        let pos = transform.translation.truncate();
        let dist = pos.distance(cursor_world);
        if dist <= hit_radius {
            if closest.is_none() || dist < closest.unwrap().1 {
                closest = Some((dot.uid, dist));
            }
        }
    }

    match closest {
        Some((uid, _)) if selected.0 == Some(uid) => selected.0 = None,
        Some((uid, _)) => selected.0 = Some(uid),
        None => selected.0 = None,
    }
}

fn update_npc_selection(
    selected: Res<SelectedNpc>,
    sim_world: Res<SimWorld>,
    mut dots: Query<(&NpcDot, &NpcFaction, &MeshMaterial2d<ColorMaterial>)>,
    mut mats: ResMut<Assets<ColorMaterial>>,
) {
    if !selected.is_changed() {
        return;
    }

    let selected_faction = selected
        .0
        .and_then(|uid| sim_world.0.npcs.get(&uid))
        .map(|npc| &npc.faction);

    for (dot, faction, mat_handle) in &mut dots {
        let color = match (selected.0, selected_faction) {
            (Some(uid), _) if uid == dot.uid => COLOR_NPC_SELECTED,
            (_, Some(sel_faction)) if *sel_faction == faction.0 => COLOR_NPC_SQUAD,
            _ => COLOR_NPC,
        };
        if let Some(m) = mats.get_mut(&mat_handle.0) {
            m.color = color;
        }
    }
}

fn deselect_on_escape(keys: Res<ButtonInput<KeyCode>>, mut selected: ResMut<SelectedNpc>) {
    if keys.just_pressed(KeyCode::Escape) {
        selected.0 = None;
    }
}
