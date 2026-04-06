//! Map tab: tooltip, zoom label, cursor-to-world helpers.

use bevy::prelude::*;
use cordon_core::primitive::tier::Tier;

use super::{LaptopFont, LaptopTab, TabContent};
use crate::PlayingState;
use crate::laptop::LaptopCamera;
use crate::laptop::input::CameraTarget;

#[derive(Component)]
pub struct ZoomLabel;

#[derive(Component)]
pub struct TooltipPanel;

#[derive(Component)]
pub struct TooltipText;

#[derive(Resource, Default)]
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

pub fn tier_color(t: &Tier) -> Color {
    match t {
        Tier::VeryLow => Color::srgb(0.5, 0.8, 0.5),
        Tier::Low => Color::srgb(0.7, 0.9, 0.4),
        Tier::Medium => Color::srgb(1.0, 0.85, 0.3),
        Tier::High => Color::srgb(1.0, 0.5, 0.2),
        Tier::VeryHigh => Color::srgb(1.0, 0.25, 0.25),
    }
}

/// Marker for UI elements that should only show on the Map tab.
#[derive(Component)]
pub struct MapOnlyUi;

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
            (follow_cursor, update_tooltip, update_zoom_label)
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
    // Tooltip panel
    commands
        .spawn((
            MapOnlyUi,
            TooltipPanel,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                min_width: Val::Px(180.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.9)),
            GlobalZIndex(100),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                TooltipText,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
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
            bottom: Val::Px(48.0),
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

fn update_tooltip(tooltip: Res<TooltipContent>, mut text_q: Query<&mut Text, With<TooltipText>>) {
    if !tooltip.is_changed() {
        return;
    }

    let content = match &*tooltip {
        TooltipContent::Hidden => String::new(),
        TooltipContent::Area {
            faction_icon,
            name,
            creatures,
            radiation,
            hazard_icon,
            loot,
            ..
        } => {
            let haz = if hazard_icon.is_empty() {
                String::new()
            } else {
                format!("  {hazard_icon}")
            };
            format!(
                "{faction_icon} {name}{haz}\nCreatures: {creatures}\nRadiation: {radiation}\nLoot: {loot}"
            )
        }
        TooltipContent::Npc {
            faction_icon,
            name,
            faction,
            rank,
            status,
        } => {
            format!("{faction_icon} {name}\nFaction: {faction}\nRank: {rank}\nStatus: {status}")
        }
    };

    for mut text in &mut text_q {
        text.0 = content.clone();
    }
}

fn update_zoom_label(
    target: Res<CameraTarget>,
    mut label_q: Query<&mut Text, (With<ZoomLabel>, Without<TooltipText>)>,
) {
    let level = (1.0 / target.zoom * 10.0).round() / 10.0;
    for mut text in &mut label_q {
        text.0 = format!("x{level:.1}");
    }
}
