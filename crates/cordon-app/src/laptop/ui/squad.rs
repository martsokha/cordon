//! Squad tab: manage hired runners and guards.

use bevy::prelude::*;

use super::{LaptopTab, TabContent};

pub struct SquadUiPlugin;

impl Plugin for SquadUiPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>, laptop_cam: Entity) {
    commands
        .spawn((
            TabContent(LaptopTab::Squad),
            bevy::ui::UiTargetCamera(laptop_cam),
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
            // Left: roster
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(40.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("SQUAD ROSTER"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    col.spawn((
                        Text::new("Runners: 0 / 0\nGuards:  0 / 0"),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                    ));
                    col.spawn((
                        Text::new("No squad members hired."),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    ));
                });

            // Right: details / missions
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(60.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("ACTIVE MISSIONS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    col.spawn((
                        Text::new("No active missions."),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    ));
                });
        });
}
