//! Upgrades tab: bunker and camp upgrade tree.

use bevy::prelude::*;

use super::{LaptopTab, TabContent};

pub struct UpgradesUiPlugin;

impl Plugin for UpgradesUiPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            TabContent(LaptopTab::Upgrades),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(90.0),
                height: Val::Percent(85.0),
                left: Val::Percent(5.0),
                top: Val::Percent(6.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::new(Val::Px(16.0), Val::Px(16.0), Val::Px(48.0), Val::Px(16.0)),
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.85)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Left: bunker upgrades
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("BUNKER"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    for name in ["Generator", "Fridge", "Stash Box", "Radio", "Workbench"] {
                        col.spawn((
                            Text::new(format!("[ ] {name}")),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                        ));
                    }
                });

            // Right: camp upgrades
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("CAMP"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    for name in [
                        "Watchtower",
                        "Antenna",
                        "Perimeter Fence",
                        "Supply Cache",
                        "Camera",
                    ] {
                        col.spawn((
                            Text::new(format!("[ ] {name}")),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                        ));
                    }
                });
        });
}
