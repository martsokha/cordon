//! Laptop UI: tab bar and tab content panels.

pub mod intel;
pub mod map;
pub mod squad;
pub mod trade;
pub mod upgrades;

use bevy::prelude::*;
use bevy_fluent::prelude::Localization;

use crate::PlayingState;

/// Which tab is currently active on the laptop.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaptopTab {
    #[default]
    Map,
    Trade,
    Squad,
    Intel,
    Upgrades,
}

/// Font handle for all laptop UI text.
#[derive(Resource)]
pub struct LaptopFont(pub Handle<Font>);

/// Marker for all 2D world entities that belong to the map view.
#[derive(Component)]
pub struct MapWorldEntity;

#[derive(Component)]
struct TabBar;

#[derive(Component)]
struct TabButton(LaptopTab);

#[derive(Component)]
struct TabContent(LaptopTab);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LaptopTab::default());
        app.add_plugins((
            map::MapUiPlugin,
            trade::TradeUiPlugin,
            squad::SquadUiPlugin,
            intel::IntelUiPlugin,
            upgrades::UpgradesUiPlugin,
        ));
        app.add_systems(Startup, load_font);
        app.add_systems(
            Update,
            (handle_tab_clicks, update_tab_visibility)
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

fn load_font(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LaptopFont(server.load("fonts/PTMono-Regular.ttf")));
}

/// Spawn the tab bar and all tab content panels.
///
/// `l10n` is used for any static label that is *authored
/// once* at spawn time (panel headings, empty-state text).
/// Runtime-updated strings still go through the usual
/// `Res<Localization>` lookup inside their refresh systems.
pub fn spawn_ui(commands: &mut Commands, font: &Handle<Font>, l10n: &Localization) {
    spawn_tab_bar(commands, font);
    map::spawn(commands, font);
    trade::spawn(commands, font);
    squad::spawn(commands, font);
    intel::spawn(commands, font, l10n);
    upgrades::spawn(commands, font);
}

fn spawn_tab_bar(commands: &mut Commands, font: &Handle<Font>) {
    // Three-slot flex layout:
    // - Left: time label (fixed-width slot so the center stays
    //   optically centered as the time string breathes).
    // - Center: tab buttons.
    // - Right: money label, right-aligned in a matching fixed slot.
    //
    // Both labels are always visible — they're HUD state, not
    // map-only overlays.
    commands
        .spawn((
            TabBar,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(32.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.95)),
            GlobalZIndex(90),
        ))
        .with_children(|bar| {
            // Left slot: time label.
            bar.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    width: Val::Px(160.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|slot| {
                slot.spawn((
                    map::TimeLabel,
                    Text::new("Day 1  08:00"),
                    TextFont {
                        font: font.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                ));
            });

            // Center slot: tab buttons.
            bar.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|slot| {
                for (tab, label) in [
                    (LaptopTab::Map, "MAP"),
                    (LaptopTab::Trade, "TRADE"),
                    (LaptopTab::Squad, "SQUAD"),
                    (LaptopTab::Intel, "INTEL"),
                    (LaptopTab::Upgrades, "UPGRADES"),
                ] {
                    slot.spawn((
                        TabButton(tab),
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.12, 0.12, 0.15, 0.9)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(label),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                        ));
                    });
                }
            });

            // Right slot: money label, right-aligned.
            bar.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    width: Val::Px(160.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|slot| {
                slot.spawn((
                    map::MoneyLabel,
                    Text::new("5000 ¢"),
                    TextFont {
                        font: font.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                ));
            });
        });
}

fn handle_tab_clicks(
    interactions: Query<(&Interaction, &TabButton), Changed<Interaction>>,
    mut active_tab: ResMut<LaptopTab>,
) {
    for (interaction, tab_btn) in &interactions {
        if *interaction == Interaction::Pressed {
            *active_tab = tab_btn.0;
        }
    }
}

fn update_tab_visibility(
    active_tab: Res<LaptopTab>,
    mut tabs: Query<(&TabContent, &mut Visibility)>,
    mut buttons: Query<(&TabButton, &mut BackgroundColor)>,
) {
    if !active_tab.is_changed() {
        return;
    }
    for (content, mut vis) in &mut tabs {
        *vis = if content.0 == *active_tab {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (btn, mut bg) in &mut buttons {
        bg.0 = if btn.0 == *active_tab {
            Color::srgba(0.2, 0.2, 0.25, 1.0)
        } else {
            Color::srgba(0.12, 0.12, 0.15, 0.9)
        };
    }
}
