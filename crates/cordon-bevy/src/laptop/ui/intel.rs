//! Intel tab: quest log, event feed, faction standings.
//!
//! The static panel layout is built once on [`spawn`]. The
//! quest-list column is tagged with [`QuestListPanel`] so
//! [`refresh_quest_list`] can clear and repopulate it whenever
//! [`QuestLog`] changes. Faction standings and the event feed
//! are still placeholder strings until their own refresh
//! systems land.

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::QuestLog;

use super::{LaptopFont, LaptopTab, TabContent};
use crate::locale::l10n_or;

/// Marker on the flex column that holds the quest-list entries.
/// Refreshed in-place by [`refresh_quest_list`].
#[derive(Component)]
pub struct QuestListPanel;

/// Marker on the heading row so [`refresh_quest_list`] knows
/// which child to preserve when rebuilding the list.
#[derive(Component)]
pub struct QuestListHeading;

pub struct IntelUiPlugin;

impl Plugin for IntelUiPlugin {
    fn build(&self, app: &mut App) {
        // Gate on the resources the refresh reads: both the
        // quest log (inserted by cordon-sim's QuestPlugin) and
        // the game data catalog (inserted once `AppState`
        // leaves `Loading`). Without these guards the system
        // runs during the loading state and panics on missing
        // `Res<GameDataResource>`.
        app.add_systems(
            Update,
            refresh_quest_list
                .run_if(resource_exists::<QuestLog>)
                .run_if(resource_exists::<GameDataResource>),
        );
    }
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            TabContent(LaptopTab::Intel),
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
            // Left: quest log — tagged for refresh.
            parent
                .spawn((
                    QuestListPanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(50.0),
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                ))
                .with_children(|col| {
                    col.spawn((
                        QuestListHeading,
                        Text::new("QUEST LOG"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                });

            // Right: faction standings + events.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("FACTION STANDINGS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    for faction in ["Garrison", "Syndicate", "Institute", "Devoted", "Drifters"] {
                        col.spawn((
                            Text::new(format!("{faction}: Neutral (0)")),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
                        ));
                    }

                    col.spawn((
                        Text::new("RECENT EVENTS"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    col.spawn((
                        Text::new("No events."),
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

/// Rebuild the quest-list column whenever [`QuestLog`] changes.
///
/// Walks the panel's children, despawns everything except the
/// heading row, then re-adds one title + one hint line per
/// active quest, plus an empty-state line when the list is
/// bare. Runs each frame the log reports `is_changed()`,
/// which in practice is once per state transition — cheap.
fn refresh_quest_list(
    mut commands: Commands,
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    l10n: Option<Res<Localization>>,
    font: Res<LaptopFont>,
    panel_q: Query<(Entity, Option<&Children>), With<QuestListPanel>>,
    heading_q: Query<(), With<QuestListHeading>>,
) {
    if !log.is_changed() {
        return;
    }
    let Ok((panel_entity, panel_children)) = panel_q.single() else {
        return;
    };

    // Despawn every child except the heading row.
    if let Some(children) = panel_children {
        for child in children.iter() {
            if heading_q.get(child).is_ok() {
                continue;
            }
            commands.entity(child).despawn();
        }
    }

    let font = font.0.clone();
    let catalog = &data.0;

    if log.active.is_empty() {
        let empty = commands
            .spawn((
                Text::new("No active quests."),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
            ))
            .id();
        commands.entity(panel_entity).add_child(empty);
        return;
    }

    for active in &log.active {
        let def_id = active.def_id.as_str();

        let title_key = format!("quest-{def_id}");
        let title = l10n
            .as_deref()
            .map(|l| l10n_or(l, &title_key, def_id))
            .unwrap_or_else(|| def_id.to_string());

        let stage_id = active.current_stage.as_str();
        let hint_key = format!("quest-{def_id}-stage-{stage_id}");
        let hint_text = l10n
            .as_deref()
            .map(|l| l10n_or(l, &hint_key, stage_id))
            .unwrap_or_else(|| stage_id.to_string());

        // One container per quest so title + hint read as a
        // single block, not as two sibling lines alternating
        // with the next quest in a flat strip.
        let entry = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(4.0), Val::Px(6.0)),
                ..default()
            })
            .id();
        commands.entity(panel_entity).add_child(entry);

        let title_node = commands
            .spawn((
                Text::new(title),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.55)),
            ))
            .id();
        commands.entity(entry).add_child(title_node);

        let hint_node = commands
            .spawn((
                Text::new(hint_text),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.85)),
            ))
            .id();
        commands.entity(entry).add_child(hint_node);

        // Catalog lookup is only for the missing-giver warning
        // — a genuine authoring error is worth flagging once.
        if let Some(def) = catalog.quests.get(&active.def_id)
            && def.giver.is_none()
        {
            warn!(
                "quest `{}` active without a giver; intel UI can still render it",
                def_id
            );
        }
    }
}
