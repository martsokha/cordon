//! Trade tab: buy/sell interface with visiting NPCs, plus the
//! daily expense report panel.

use bevy::prelude::*;
use cordon_core::entity::player::ExpenseKind;
use cordon_sim::plugin::prelude::{LastDailyExpenses, Player};

use super::{LaptopFont, LaptopTab, TabContent};
use crate::PlayingState;

#[derive(Component)]
struct ExpensePanel;

pub struct TradeUiPlugin;

impl Plugin for TradeUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            refresh_expense_panel.run_if(in_state(PlayingState::Laptop)),
        );
    }
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
            // Left panel: visitors + daily expenses.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(30.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|col| {
                    // Visitors list (top).
                    col.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|top| {
                        top.spawn((
                            Text::new("VISITORS"),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.6)),
                        ));
                        top.spawn((
                            Text::new("No visitors at the counter."),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                        ));
                    });

                    // Daily expenses (bottom).
                    col.spawn((
                        ExpensePanel,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            padding: UiRect::top(Val::Px(8.0)),
                            border: UiRect::top(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.4, 0.4, 0.3, 0.4)),
                    ));
                });

            // Right panel: trade details.
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

fn refresh_expense_panel(
    mut commands: Commands,
    expenses: Res<LastDailyExpenses>,
    player: Res<Player>,
    font: Res<LaptopFont>,
    panel_q: Query<(Entity, Option<&Children>), With<ExpensePanel>>,
) {
    if !expenses.is_changed() && !player.is_changed() {
        return;
    }
    let Ok((panel, children)) = panel_q.single() else {
        return;
    };

    // Clear existing children.
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let font_handle = font.0.clone();
    let dim = Color::srgba(0.5, 0.5, 0.5, 0.8);
    let normal = Color::srgb(0.75, 0.75, 0.7);
    let red = Color::srgb(0.9, 0.3, 0.3);

    let heading = spawn_text(&mut commands, "DAILY EXPENSES", 11.0, &font_handle, Color::srgb(0.8, 0.8, 0.6));
    commands.entity(panel).add_child(heading);

    let Some(report) = &expenses.0 else {
        let none = spawn_text(&mut commands, "No data yet.", 10.0, &font_handle, dim);
        commands.entity(panel).add_child(none);
        return;
    };

    for line in &report.lines {
        let label = match line.kind {
            ExpenseKind::SquadUpkeep => "Squad upkeep",
            ExpenseKind::GarrisonBribe => "Garrison bribe",
            ExpenseKind::SyndicateInterest => "Syndicate interest",
        };
        let row = spawn_expense_row(
            &mut commands,
            label,
            line.amount.value(),
            &font_handle,
            normal,
        );
        commands.entity(panel).add_child(row);
    }

    // Separator + total.
    let sep = commands
        .spawn(Node {
            height: Val::Px(1.0),
            width: Val::Percent(100.0),
            margin: UiRect::vertical(Val::Px(2.0)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.4, 0.4, 0.3, 0.3)))
        .id();
    commands.entity(panel).add_child(sep);

    let total_row = spawn_expense_row(
        &mut commands,
        "Total",
        report.total.value(),
        &font_handle,
        normal,
    );
    commands.entity(panel).add_child(total_row);

    // Debt line if any.
    let debt = player.0.debt.value();
    if debt > 0 {
        let debt_row = spawn_expense_row(
            &mut commands,
            "Outstanding debt",
            debt,
            &font_handle,
            red,
        );
        commands.entity(panel).add_child(debt_row);
    }

    // Balance.
    let bal_color = if player.0.credits.value() == 0 && debt > 0 {
        red
    } else {
        normal
    };
    let bal_row = spawn_expense_row(
        &mut commands,
        "Balance",
        player.0.credits.value(),
        &font_handle,
        bal_color,
    );
    commands.entity(panel).add_child(bal_row);
}

fn spawn_text(
    commands: &mut Commands,
    text: &str,
    size: f32,
    font: &Handle<Font>,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextFont {
                font: font.clone(),
                font_size: size,
                ..default()
            },
            TextColor(color),
        ))
        .id()
}

fn spawn_expense_row(
    commands: &mut Commands,
    label: &str,
    amount: u32,
    font: &Handle<Font>,
    color: Color,
) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: Val::Percent(100.0),
            ..default()
        })
        .id();
    let left = spawn_text(commands, label, 10.0, font, color);
    let right = spawn_text(commands, &format!("{amount} ¤"), 10.0, font, color);
    commands.entity(row).add_children(&[left, right]);
    row
}
