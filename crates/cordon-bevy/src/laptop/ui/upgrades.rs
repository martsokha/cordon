//! Upgrades tab: data-driven list of bunker + camp upgrades with
//! cost, status, and click-to-install rows.
//!
//! Layout is built once at laptop-UI spawn; the row content is
//! rebuilt by [`refresh_upgrade_panels`] whenever `Player` or
//! the game data changes, so credits spent / new installs
//! reflect immediately.

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::entity::bunker::{Upgrade, UpgradeDef, UpgradeLocation};
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::BuyUpgrade;
use cordon_sim::resources::{PlayerIdentity, PlayerUpgrades};

use super::{LaptopFont, LaptopTab, TabContent};
use crate::PlayingState;
use crate::locale::l10n_or;

/// Marker for the BUNKER column's row container.
#[derive(Component)]
struct BunkerUpgradesPanel;

/// Marker for the CAMP column's row container.
#[derive(Component)]
struct CampUpgradesPanel;

/// Heading marker: preserved during rebuild so the column label
/// doesn't flash on every refresh.
#[derive(Component)]
struct UpgradeColumnHeading;

/// Attached to each interactive row button — carries the id so
/// click handling can dispatch the right [`BuyUpgrade`].
#[derive(Component)]
struct UpgradeBuyButton {
    upgrade: Id<Upgrade>,
}

pub struct UpgradesUiPlugin;

impl Plugin for UpgradesUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (refresh_upgrade_panels, handle_buy_clicks).run_if(in_state(PlayingState::Laptop)),
        );
    }
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
            spawn_bunker_column(parent, font);
            spawn_camp_column(parent, font);
        });
}

fn spawn_bunker_column(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            BunkerUpgradesPanel,
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(50.0),
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
        ))
        .with_children(|col| spawn_heading(col, font, "BUNKER"));
}

fn spawn_camp_column(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            CampUpgradesPanel,
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(50.0),
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
        ))
        .with_children(|col| spawn_heading(col, font, "CAMP"));
}

fn spawn_heading(col: &mut ChildSpawnerCommands, font: &Handle<Font>, text: &str) {
    col.spawn((
        UpgradeColumnHeading,
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.6)),
    ));
}

/// Rebuild both upgrade lists when the player state or catalog
/// changes. First-fill handling: if a panel has only its
/// heading child, we rebuild once unconditionally.
#[allow(clippy::type_complexity)]
fn refresh_upgrade_panels(
    mut commands: Commands,
    identity: Res<PlayerIdentity>,
    upgrades: Res<PlayerUpgrades>,
    data: Res<GameDataResource>,
    l10n: Option<Res<Localization>>,
    font: Res<LaptopFont>,
    bunker_q: Query<
        (Entity, Option<&Children>),
        (With<BunkerUpgradesPanel>, Without<CampUpgradesPanel>),
    >,
    camp_q: Query<
        (Entity, Option<&Children>),
        (With<CampUpgradesPanel>, Without<BunkerUpgradesPanel>),
    >,
    heading_q: Query<(), With<UpgradeColumnHeading>>,
) {
    let bunker_needs_fill = bunker_q
        .iter()
        .any(|(_, c)| non_heading_count(c, &heading_q) == 0);
    let camp_needs_fill = camp_q
        .iter()
        .any(|(_, c)| non_heading_count(c, &heading_q) == 0);
    let dirty = identity.is_changed() || upgrades.is_changed() || data.is_changed();

    if !dirty && !bunker_needs_fill && !camp_needs_fill {
        return;
    }
    let Some(l10n) = l10n else {
        return;
    };

    let mut bunker_defs: Vec<&UpgradeDef> = Vec::new();
    let mut camp_defs: Vec<&UpgradeDef> = Vec::new();
    for def in data.0.upgrades.values() {
        match def.location {
            UpgradeLocation::Bunker => bunker_defs.push(def),
            UpgradeLocation::Camp => camp_defs.push(def),
        }
    }
    bunker_defs.sort_by_key(|d| (d.cost.value(), d.id.as_str().to_string()));
    camp_defs.sort_by_key(|d| (d.cost.value(), d.id.as_str().to_string()));

    for (panel, children) in &bunker_q {
        clear_non_heading(&mut commands, children, &heading_q);
        for def in &bunker_defs {
            commands.entity(panel).with_children(|col| {
                spawn_row(col, &font.0, &l10n, def, &identity, &upgrades);
            });
        }
    }
    for (panel, children) in &camp_q {
        clear_non_heading(&mut commands, children, &heading_q);
        for def in &camp_defs {
            commands.entity(panel).with_children(|col| {
                spawn_row(col, &font.0, &l10n, def, &identity, &upgrades);
            });
        }
    }
}

fn non_heading_count(
    children: Option<&Children>,
    heading_q: &Query<(), With<UpgradeColumnHeading>>,
) -> usize {
    children
        .map(|c| c.iter().filter(|e| heading_q.get(*e).is_err()).count())
        .unwrap_or(0)
}

fn clear_non_heading(
    commands: &mut Commands,
    children: Option<&Children>,
    heading_q: &Query<(), With<UpgradeColumnHeading>>,
) {
    if let Some(children) = children {
        for child in children.iter() {
            if heading_q.get(child).is_err() {
                commands.entity(child).despawn();
            }
        }
    }
}

/// Spawn one upgrade row: name + cost + status, with a button
/// that dispatches [`BuyUpgrade`] when available.
fn spawn_row(
    col: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    l10n: &Localization,
    def: &UpgradeDef,
    identity: &PlayerIdentity,
    upgrades: &PlayerUpgrades,
) {
    let name_key = format!("{}-name", def.id.as_str().replace('_', "-"));
    let name = l10n_or(l10n, &name_key, def.id.as_str());

    let installed = upgrades.has_upgrade(&def.id);
    let prereqs_met = def.requires.iter().all(|r| upgrades.has_upgrade(r));
    let affordable = identity.credits.value() >= def.cost.value();

    let (status_text, status_color) = if installed {
        ("INSTALLED", Color::srgb(0.5, 0.9, 0.5))
    } else if !prereqs_met {
        ("LOCKED", Color::srgb(0.5, 0.5, 0.5))
    } else if !affordable {
        ("NEED CREDITS", Color::srgb(0.9, 0.7, 0.4))
    } else {
        ("AVAILABLE", Color::srgb(0.7, 0.9, 1.0))
    };

    let row_bg = if installed {
        Color::srgba(0.08, 0.15, 0.08, 0.6)
    } else {
        Color::srgba(0.08, 0.08, 0.12, 0.6)
    };

    // Only clickable when it's actually purchasable.
    let clickable = !installed && prereqs_met && affordable;

    col.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(6.0)),
            column_gap: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(row_bg),
    ))
    .with_children(|row| {
        // Left: name + cost.
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|info| {
            info.spawn((
                Text::new(name),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.85)),
            ));
            info.spawn((
                Text::new(format!("{} cr", def.cost.value())),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });

        // Right: status pill / buy button.
        let mut entity = row.spawn((
            Node {
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
        ));
        if clickable {
            entity.insert((
                Button,
                UpgradeBuyButton {
                    upgrade: def.id.clone(),
                },
            ));
        }
        entity.with_children(|pill| {
            pill.spawn((
                Text::new(status_text),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(status_color),
            ));
        });
    });
}

fn handle_buy_clicks(
    interaction_q: Query<(&Interaction, &UpgradeBuyButton), Changed<Interaction>>,
    mut writer: MessageWriter<BuyUpgrade>,
) {
    for (interaction, button) in &interaction_q {
        if matches!(interaction, Interaction::Pressed) {
            writer.write(BuyUpgrade {
                upgrade: button.upgrade.clone(),
            });
        }
    }
}
