//! Trade tab: buy/sell interface with visiting NPCs.

use bevy::prelude::*;

use super::{LaptopTab, TabContent};

pub struct TradeUiPlugin;

impl Plugin for TradeUiPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            TabContent(LaptopTab::Trade),
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
            // Left panel: NPC list
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(30.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("VISITORS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    col.spawn((
                        Text::new("No visitors at the counter."),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                    ));
                });

            // Right panel: trade details
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(70.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("SELECT A VISITOR TO TRADE"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    ));
                });
        });
}
