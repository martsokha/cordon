//! Map tab: tooltip, zoom label, cursor-to-world helpers.

use bevy::prelude::*;
use cordon_core::primitive::Tier;
use cordon_sim::resources::GameClock;

use super::LaptopTab;
use crate::PlayingState;
use crate::laptop::LaptopCamera;
use crate::laptop::input::CameraTarget;

#[derive(Component)]
pub struct ZoomLabel;

#[derive(Component)]
pub struct TimeLabel;

#[derive(Component)]
pub struct TooltipPanel;

#[derive(Component)]
pub struct TooltipRoot;

#[derive(Component)]
struct TooltipHeader;

#[derive(Component)]
struct TooltipIcons;

#[derive(Resource, Default, Clone)]
pub enum TooltipContent {
    #[default]
    Hidden,
    Area {
        faction_icon: String,
        name: String,
        creatures: String,
        creatures_tier: Tier,
        radiation: String,
        radiation_tier: Tier,
        hazard_image: Option<String>,
        hazard_count: u8,
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
    Relic {
        name: String,
        origin: String,
        rarity: String,
        /// Pre-formatted lines like `"Ballistic: +20"` / `"Health: +2 max"`.
        passives: Vec<String>,
        /// Number of reactive effects (OnHit / OnHpLow / Periodic).
        triggered_count: usize,
    },
}

/// Marker for UI elements that should only show on the Map tab.
#[derive(Component)]
pub struct MapOnlyUi;

fn tier_color(t: &Tier) -> Color {
    match t {
        Tier::VeryLow => Color::srgb(0.5, 0.8, 0.5),
        Tier::Low => Color::srgb(0.7, 0.9, 0.4),
        Tier::Medium => Color::srgb(1.0, 0.85, 0.3),
        Tier::High => Color::srgb(1.0, 0.5, 0.2),
        Tier::VeryHigh => Color::srgb(1.0, 0.25, 0.25),
    }
}

const COLOR_LABEL: Color = Color::srgba(0.6, 0.6, 0.6, 1.0);

pub struct MapUiPlugin;

impl Plugin for MapUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TooltipContent::default());
        app.add_systems(
            Update,
            update_map_ui_visibility.run_if(in_state(PlayingState::Laptop)),
        );
        app.add_systems(
            Update,
            (
                follow_cursor,
                update_tooltip,
                update_zoom_label,
                update_time_label,
            )
                .run_if(in_state(PlayingState::Laptop))
                .run_if(resource_equals(LaptopTab::Map)),
        );
    }
}

fn update_map_ui_visibility(
    active_tab: Res<LaptopTab>,
    mut ui_q: Query<&mut Visibility, (With<MapOnlyUi>, Without<super::MapWorldEntity>)>,
    mut world_q: Query<&mut Visibility, (With<super::MapWorldEntity>, Without<MapOnlyUi>)>,
) {
    if !active_tab.is_changed() {
        return;
    }
    let visible = *active_tab == LaptopTab::Map;
    for mut vis in &mut ui_q {
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut vis in &mut world_q {
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Get cursor position in 2D world space via the laptop camera.
pub fn cursor_world_pos(
    windows: &Query<&Window>,
    cameras: &Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor_screen = window.cursor_position()?;
    let (camera, cam_transform) = cameras.iter().next()?;
    camera
        .viewport_to_world_2d(cam_transform, cursor_screen)
        .ok()
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>) {
    // Tooltip panel with a single Text root + TextSpan children
    commands
        .spawn((
            MapOnlyUi,
            TooltipPanel,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(2.0),
                min_width: Val::Px(200.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.92)),
            GlobalZIndex(100),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            // Header row: name text + hazard icons
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    TooltipHeader,
                    Text::new(""),
                    TextFont {
                        font: font.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                row.spawn((
                    TooltipIcons,
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(2.0),
                        ..default()
                    },
                ));
            });
            // Stat rows as TextSpan
            p.spawn((
                TooltipRoot,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(COLOR_LABEL),
            ));
        });

    // Zoom label
    commands.spawn((
        MapOnlyUi,
        ZoomLabel,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
            ..default()
        },
        Text::new("x1.0"),
        TextFont {
            font: font.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
    ));

    // Time display
    commands.spawn((
        MapOnlyUi,
        TimeLabel,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(48.0),
            ..default()
        },
        Text::new("Day 1  08:00"),
        TextFont {
            font: font.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
    ));
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

fn update_tooltip(
    tooltip: Res<TooltipContent>,
    root_q: Query<(Entity, Option<&Children>), With<TooltipRoot>>,
    mut header_q: Query<&mut Text, (With<TooltipHeader>, Without<TooltipRoot>)>,
    icons_q: Query<(Entity, Option<&Children>), (With<TooltipIcons>, Without<TooltipRoot>)>,
    mut commands: Commands,
    font: Option<Res<super::LaptopFont>>,
    asset_server: Res<AssetServer>,
) {
    if !tooltip.is_changed() {
        return;
    }
    let Some(font) = font else { return };
    let Ok((root_entity, root_children)) = root_q.single() else {
        return;
    };
    let f = font.0.clone();

    // Despawn old text span children
    if let Some(children) = root_children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    // Extract data before mutating
    let header_text;
    let hazard_img: Option<String>;
    let hazard_n: u8;
    let spans: Vec<(String, Color)>;

    match &*tooltip {
        TooltipContent::Hidden => {
            header_text = String::new();
            hazard_img = None;
            hazard_n = 0;
            spans = vec![];
        }
        TooltipContent::Area {
            faction_icon,
            name,
            creatures,
            creatures_tier,
            radiation,
            radiation_tier,
            hazard_image,
            hazard_count,
            loot,
            loot_tier,
        } => {
            header_text = format!("{faction_icon} {name}");
            hazard_img = hazard_image.clone();
            hazard_n = *hazard_count;
            spans = vec![
                ("Creatures: ".into(), COLOR_LABEL),
                (creatures.clone(), tier_color(creatures_tier)),
                ("\nRadiation: ".into(), COLOR_LABEL),
                (radiation.clone(), tier_color(radiation_tier)),
                ("\nLoot: ".into(), COLOR_LABEL),
                (loot.clone(), tier_color(loot_tier)),
            ];
        }
        TooltipContent::Npc {
            faction_icon,
            name,
            faction,
            rank,
            status,
        } => {
            header_text = format!("{faction_icon} {name}");
            hazard_img = None;
            hazard_n = 0;
            spans = vec![
                ("Faction: ".into(), COLOR_LABEL),
                (faction.clone(), Color::srgb(0.7, 0.7, 0.7)),
                ("\nRank: ".into(), COLOR_LABEL),
                (rank.clone(), Color::srgb(0.8, 0.8, 0.6)),
                ("\nStatus: ".into(), COLOR_LABEL),
                (status.clone(), COLOR_LABEL),
            ];
        }
        TooltipContent::Relic {
            name,
            origin,
            rarity,
            passives,
            triggered_count,
        } => {
            header_text = format!("* {name}");
            hazard_img = None;
            hazard_n = 0;
            let mut s: Vec<(String, Color)> = vec![
                ("Origin: ".into(), COLOR_LABEL),
                (origin.clone(), Color::srgb(0.3, 0.9, 1.0)),
                ("\nRarity: ".into(), COLOR_LABEL),
                (rarity.clone(), Color::srgb(0.8, 0.8, 0.6)),
            ];
            for line in passives {
                s.push(("\n".into(), COLOR_LABEL));
                s.push((line.clone(), Color::srgb(0.6, 0.9, 0.6)));
            }
            if *triggered_count > 0 {
                s.push(("\n+ ".into(), COLOR_LABEL));
                s.push((
                    format!(
                        "{triggered_count} reactive effect{}",
                        if *triggered_count == 1 { "" } else { "s" }
                    ),
                    Color::srgb(0.9, 0.7, 0.3),
                ));
            }
            spans = s;
        }
    }

    // Update header text
    for mut text in &mut header_q {
        text.0 = header_text.clone();
    }

    // Update hazard icons
    if let Ok((icons_entity, icon_children)) = icons_q.single() {
        if let Some(children) = icon_children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(icons_entity).detach_all_children();
        if let Some(path) = &hazard_img {
            let img: Handle<Image> = asset_server.load(path.clone());
            for _ in 0..hazard_n {
                let icon = commands
                    .spawn((
                        ImageNode {
                            image: img.clone(),
                            ..default()
                        },
                        Node {
                            width: Val::Px(14.0),
                            height: Val::Px(14.0),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(icons_entity).add_child(icon);
            }
        }
    }

    // Update stat spans
    commands
        .entity(root_entity)
        .insert(Text::new(""))
        .insert(TextFont {
            font: f.clone(),
            font_size: 11.0,
            ..default()
        })
        .insert(TextColor(COLOR_LABEL));
    commands.entity(root_entity).detach_all_children();

    for (text, color) in &spans {
        let span = commands
            .spawn((
                TextSpan::new(text.clone()),
                TextFont {
                    font: f.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(*color),
            ))
            .id();
        commands.entity(root_entity).add_child(span);
    }
}

fn update_zoom_label(
    target: Res<CameraTarget>,
    mut label_q: Query<&mut Text, (With<ZoomLabel>, Without<TooltipRoot>, Without<TimeLabel>)>,
) {
    use crate::laptop::input::{ZOOM_MAX, ZOOM_MIN};
    let t = (target.zoom - ZOOM_MIN) / (ZOOM_MAX - ZOOM_MIN);
    let level = ((1.0 - t) * 3.0 + 1.0).clamp(1.0, 4.0);
    for mut text in &mut label_q {
        text.0 = format!("x{level:.1}");
    }
}

fn update_time_label(
    clock: Option<Res<GameClock>>,
    mut label_q: Query<&mut Text, (With<TimeLabel>, Without<TooltipRoot>, Without<ZoomLabel>)>,
) {
    let Some(clock) = clock else { return };
    let t = &clock.0;
    for mut text in &mut label_q {
        text.0 = format!("Day {}  {}", t.day.value(), t.time_str());
    }
}
