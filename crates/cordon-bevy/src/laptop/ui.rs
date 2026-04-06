//! Laptop tooltip UI: panel, follow cursor, update content.

use bevy::prelude::*;
use cordon_core::primitive::tier::Tier;

use super::input::CameraTarget;
use crate::PlayingState;

/// Font handle for all laptop UI text.
#[derive(Resource)]
pub struct LaptopFont(pub Handle<Font>);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TooltipContent::default());
        app.add_systems(Startup, load_font);
        app.add_systems(
            Update,
            (follow_cursor, update_tooltip_ui, update_zoom_label)
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

fn load_font(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LaptopFont(server.load("fonts/PTMono-Regular.ttf")));
}

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

#[derive(Component)]
pub struct TooltipPanel;

#[derive(Component)]
pub struct TtHeader;

#[derive(Component)]
pub struct TtRow1Label;
#[derive(Component)]
pub struct TtRow1Value;

#[derive(Component)]
pub struct TtRow2Label;
#[derive(Component)]
pub struct TtRow2Value;

#[derive(Component)]
pub struct TtHazardIcon;

#[derive(Component)]
pub struct ZoomLabel;

#[derive(Component)]
pub struct TtRow3Label;
#[derive(Component)]
pub struct TtRow3Value;

pub const COLOR_LABEL: Color = Color::srgba(0.6, 0.6, 0.6, 1.0);

pub fn tier_color(t: &Tier) -> Color {
    match t {
        Tier::VeryLow => Color::srgb(0.5, 0.8, 0.5),
        Tier::Low => Color::srgb(0.7, 0.9, 0.4),
        Tier::Medium => Color::srgb(1.0, 0.85, 0.3),
        Tier::High => Color::srgb(1.0, 0.5, 0.2),
        Tier::VeryHigh => Color::srgb(1.0, 0.25, 0.25),
    }
}

/// Spawn the tooltip panel as a UI entity. Call from spawn_map.
pub fn spawn_tooltip_panel(commands: &mut Commands, font: &Handle<Font>) {
    let hdr_font = TextFont {
        font: font.clone(),
        font_size: 14.0,
        ..default()
    };
    let lbl_font = TextFont {
        font: font.clone(),
        font_size: 11.0,
        ..default()
    };
    let val_font = TextFont {
        font: font.clone(),
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

    commands.spawn((
        ZoomLabel,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            bottom: Val::Px(12.0),
            ..default()
        },
        Text::new("x1.0"),
        TextFont {
            font: font.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
    ));
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

fn update_zoom_label(target: Res<CameraTarget>, mut label_q: Query<&mut Text, With<ZoomLabel>>) {
    if !target.is_changed() {
        return;
    }
    let level = (1.0 / target.zoom * 10.0).round() / 10.0;
    for mut text in &mut label_q {
        text.0 = format!("x{level:.1}");
    }
}
