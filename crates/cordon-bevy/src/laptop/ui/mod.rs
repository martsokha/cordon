//! Laptop UI: tab bar and tab content panels.

pub mod intel;
pub mod map;
pub mod squad;
pub mod trade;
pub mod upgrades;

use bevy::prelude::*;

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
pub fn spawn_ui(commands: &mut Commands, font: &Handle<Font>) {
    spawn_tab_bar(commands, font);
    map::spawn(commands, font);
    trade::spawn(commands, font);
    squad::spawn(commands, font);
    intel::spawn(commands, font);
    upgrades::spawn(commands, font);
}

fn spawn_tab_bar(commands: &mut Commands, font: &Handle<Font>) {
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
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(2.0),
                padding: UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.95)),
            GlobalZIndex(90),
        ))
        .with_children(|bar| {
            for (tab, label) in [
                (LaptopTab::Map, "MAP"),
                (LaptopTab::Trade, "TRADE"),
                (LaptopTab::Squad, "SQUAD"),
                (LaptopTab::Intel, "INTEL"),
                (LaptopTab::Upgrades, "UPGRADES"),
            ] {
                bar.spawn((
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
