//! Laptop UI: tab bar and tab content panels.

pub mod intel;
pub mod map;
pub mod squad;
pub mod trade;
pub mod upgrades;

use bevy::prelude::*;

use crate::PlayingState;
use crate::locale::L10n;

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

/// Marker for all 2D world entities that belong to the map view.
#[derive(Component)]
pub struct MapWorldEntity;

/// Marker for UI chrome (tab bar, crosshair, zoom label, squad
/// roster, tooltip, non-map tabs) that should only appear while
/// the player is in `PlayingState::Laptop` fullscreen. The desk
/// projection shows only the map content itself.
#[derive(Component)]
pub struct LaptopChromeUi;

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
        app.add_systems(
            Update,
            (handle_tab_clicks, update_tab_visibility)
                .chain()
                .run_if(in_state(PlayingState::Laptop)),
        );
        // Chrome UI (tab bar, crosshair, squad roster, etc.) is
        // only shown in fullscreen laptop mode. The desk
        // projection shows just the active tab's content. Gated
        // on `resource_exists` because `PlayingState` is a
        // sub-state of `AppState::Playing` and won't be in the
        // world during loading / menu.
        app.add_systems(
            Update,
            sync_chrome_visibility.run_if(resource_exists::<State<PlayingState>>),
        );
    }
}

/// Sole owner of chrome UI visibility. Computes the target
/// visibility from (state, tab, MapOnlyUi) every frame the
/// inputs change — so both "player enters laptop mode" and
/// "player switches tabs" land here instead of being handled
/// by two different systems that fight each other.
fn sync_chrome_visibility(
    state: Res<State<PlayingState>>,
    active_tab: Res<LaptopTab>,
    mut chrome_q: Query<(&mut Visibility, Has<map::MapOnlyUi>), With<LaptopChromeUi>>,
) {
    if !state.is_changed() && !active_tab.is_changed() {
        return;
    }
    let in_laptop = matches!(state.get(), PlayingState::Laptop);
    let on_map_tab = *active_tab == LaptopTab::Map;
    for (mut vis, is_map_only) in &mut chrome_q {
        // Chrome visibility is the AND of:
        //   - player is in fullscreen laptop mode
        //   - if the chrome is MapOnlyUi, the Map tab is active
        let target = if in_laptop && (!is_map_only || on_map_tab) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != target {
            *vis = target;
        }
    }
}

/// Spawn the tab bar and all tab content panels.
///
/// `l10n` is used for any static label that is *authored
/// once* at spawn time (panel headings, empty-state text).
/// Runtime-updated strings still go through the usual
/// `L10n` lookup inside their refresh systems.
pub fn spawn_ui(commands: &mut Commands, font: &Handle<Font>, l10n: &L10n, laptop_cam: Entity) {
    spawn_tab_bar(commands, font, laptop_cam);
    map::spawn(commands, font, laptop_cam);
    trade::spawn(commands, font, laptop_cam);
    squad::spawn(commands, font, laptop_cam);
    intel::spawn(commands, font, l10n, laptop_cam);
    upgrades::spawn(commands, font, laptop_cam);
}

fn spawn_tab_bar(commands: &mut Commands, font: &Handle<Font>, laptop_cam: Entity) {
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
            LaptopChromeUi,
            bevy::ui::UiTargetCamera(laptop_cam),
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
            Visibility::Hidden,
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
