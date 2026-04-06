//! Map tab: tooltip, zoom label, cursor-to-world helpers.

use bevy::prelude::*;
use cordon_core::primitive::Tier;

use super::LaptopTab;
use crate::PlayingState;
use crate::laptop::LaptopCamera;
use crate::laptop::input::CameraTarget;
use crate::world::SimWorld;

#[derive(Component)]
pub struct ZoomLabel;

#[derive(Component)]
pub struct TimeLabel;

#[derive(Component)]
pub struct TooltipPanel;

#[derive(Component)]
pub struct TooltipRoot;

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
        hazard_icon: String,
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
            (follow_cursor, update_tooltip, update_zoom_label, update_time_label)
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
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in &mut world_q {
        *vis = if visible { Visibility::Visible } else { Visibility::Hidden };
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
    camera.viewport_to_world_2d(cam_transform, cursor_screen).ok()
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
            p.spawn((
                TooltipRoot,
                Text::new(""),
                TextFont { font: font.clone(), font_size: 12.0, ..default() },
                TextColor(Color::WHITE),
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
        TextFont { font: font.clone(), font_size: 12.0, ..default() },
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
        TextFont { font: font.clone(), font_size: 12.0, ..default() },
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
    root_q: Query<Entity, With<TooltipRoot>>,
    mut commands: Commands,
    font: Option<Res<super::LaptopFont>>,
) {
    if !tooltip.is_changed() {
        return;
    }
    let Some(font) = font else { return };
    let Ok(root_entity) = root_q.single() else { return };
    let f = font.0.clone();

    // Build spans: (text, font_size, color)
    let spans: Vec<(String, f32, Color)> = match &*tooltip {
        TooltipContent::Hidden => vec![],
        TooltipContent::Area {
            faction_icon, name, creatures, creatures_tier,
            radiation, radiation_tier, hazard_icon, hazard_image: _, hazard_count,
            loot, loot_tier,
        } => {
            let haz = if *hazard_count > 0 {
                format!(" {}", hazard_icon.repeat(*hazard_count as usize))
            } else {
                String::new()
            };
            vec![
                (format!("{faction_icon} {name}{haz}"), 13.0, Color::WHITE),
                (format!("\nCreatures: "), 11.0, COLOR_LABEL),
                (creatures.clone(), 11.0, tier_color(creatures_tier)),
                (format!("\nRadiation: "), 11.0, COLOR_LABEL),
                (radiation.clone(), 11.0, tier_color(radiation_tier)),
                (format!("\nLoot: "), 11.0, COLOR_LABEL),
                (loot.clone(), 11.0, tier_color(loot_tier)),
            ]
        }
        TooltipContent::Npc {
            faction_icon, name, faction, rank, status,
        } => {
            vec![
                (format!("{faction_icon} {name}"), 13.0, Color::WHITE),
                ("\nFaction: ".into(), 11.0, COLOR_LABEL),
                (faction.clone(), 11.0, Color::srgb(0.7, 0.7, 0.7)),
                ("\nRank: ".into(), 11.0, COLOR_LABEL),
                (rank.clone(), 11.0, Color::srgb(0.8, 0.8, 0.6)),
                ("\nStatus: ".into(), 11.0, COLOR_LABEL),
                (status.clone(), 11.0, COLOR_LABEL),
            ]
        }
    };

    // Set root text to first span (or empty)
    if let Some((first_text, first_size, first_color)) = spans.first() {
        commands.entity(root_entity)
            .insert(Text::new(first_text.clone()))
            .insert(TextFont { font: f.clone(), font_size: *first_size, ..default() })
            .insert(TextColor(*first_color));
    } else {
        commands.entity(root_entity).insert(Text::new(""));
    }

    // Remove old TextSpan children
    commands.entity(root_entity).clear_children();

    // Add remaining spans as children
    for (text, size, color) in spans.iter().skip(1) {
        let span = commands.spawn((
            TextSpan::new(text.clone()),
            TextFont { font: f.clone(), font_size: *size, ..default() },
            TextColor(*color),
        )).id();
        commands.entity(root_entity).add_child(span);
    }
}

#[allow(clippy::type_complexity)]
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

#[allow(clippy::type_complexity)]
fn update_time_label(
    sim: Option<Res<SimWorld>>,
    mut label_q: Query<&mut Text, (With<TimeLabel>, Without<TooltipRoot>, Without<ZoomLabel>)>,
) {
    let Some(sim) = sim else { return };
    let t = &sim.0.time;
    for mut text in &mut label_q {
        text.0 = format!("Day {}  {}", t.day.value(), t.time_str());
    }
}
