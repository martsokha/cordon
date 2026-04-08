//! Intel tab: quest log, event feed, faction standings.
//!
//! The static panel layout is built once on [`spawn`]. Two
//! containers are tagged with marker components so per-frame
//! refresh systems can rebuild their children from live world
//! state:
//!
//! - [`QuestListPanel`] — rebuilt by [`refresh_quest_list`]
//!   whenever [`QuestLog`] changes.
//! - [`FactionStandingsPanel`] — rebuilt by
//!   [`refresh_faction_standings`] whenever [`Player`] changes.
//!
//! The event feed is still a placeholder until an event log
//! refresh system lands.

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{Id, Relation};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::{Player, QuestLog};

use super::{LaptopFont, LaptopTab, TabContent};
use crate::locale::l10n_or;

/// Marker on the flex column that holds the quest-list entries.
/// Refreshed in-place by [`refresh_quest_list`].
#[derive(Component)]
pub struct QuestListPanel;

/// Marker on the heading row of the quest list so
/// [`refresh_quest_list`] knows which child to preserve when
/// rebuilding the list.
#[derive(Component)]
pub struct QuestListHeading;

/// Marker on the flex column that holds the faction-standings
/// rows. Refreshed in-place by [`refresh_faction_standings`].
#[derive(Component)]
pub struct FactionStandingsPanel;

/// Marker on the heading row of the faction standings so
/// [`refresh_faction_standings`] knows which child to preserve
/// when rebuilding the list.
#[derive(Component)]
pub struct FactionStandingsHeading;

pub struct IntelUiPlugin;

impl Plugin for IntelUiPlugin {
    fn build(&self, app: &mut App) {
        // Gate on the resources the refreshes read: the quest
        // log + player (inserted by cordon-sim's plugins) and
        // the game data catalog (inserted once `AppState`
        // leaves `Loading`). Without these guards the systems
        // run during the loading state and panic on missing
        // resources.
        app.add_systems(
            Update,
            (refresh_quest_list, refresh_faction_standings)
                .run_if(resource_exists::<QuestLog>)
                .run_if(resource_exists::<Player>)
                .run_if(resource_exists::<GameDataResource>),
        );
    }
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>, l10n: &Localization) {
    let quest_log_heading = l10n_or(l10n, "intel-quest-log", "QUEST LOG");
    let faction_standings_heading = l10n_or(l10n, "intel-faction-standings", "FACTION STANDINGS");
    let recent_events_heading = l10n_or(l10n, "intel-recent-events", "RECENT EVENTS");
    let events_empty = l10n_or(l10n, "intel-events-empty", "No events.");

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
                        Text::new(quest_log_heading),
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
                    // Faction standings: heading inside a tagged
                    // column so the refresh system owns the row
                    // children without touching the event feed
                    // below.
                    col.spawn((
                        FactionStandingsPanel,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            FactionStandingsHeading,
                            Text::new(faction_standings_heading),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.6)),
                        ));
                    });

                    col.spawn((
                        Text::new(recent_events_heading),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.6)),
                    ));
                    col.spawn((
                        Text::new(events_empty),
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

/// Rebuild the quest-list column whenever [`QuestLog`] changes,
/// or on the first frame the panel exists (same lifecycle
/// caveat as [`refresh_faction_standings`]: `QuestLog` is
/// inserted by cordon-sim's `QuestPlugin` before the laptop UI
/// is spawned, so the `is_changed()` window has already closed
/// by the time this system first gets a panel to populate).
fn refresh_quest_list(
    mut commands: Commands,
    log: Res<QuestLog>,
    data: Res<GameDataResource>,
    l10n: Option<Res<Localization>>,
    font: Res<LaptopFont>,
    panel_q: Query<(Entity, Option<&Children>), With<QuestListPanel>>,
    heading_q: Query<(), With<QuestListHeading>>,
) {
    let Ok((panel_entity, panel_children)) = panel_q.single() else {
        return;
    };
    let non_heading_count = panel_children
        .map(|children| {
            children
                .iter()
                .filter(|c| heading_q.get(*c).is_err())
                .count()
        })
        .unwrap_or(0);
    let needs_first_fill = non_heading_count == 0;
    if !log.is_changed() && !needs_first_fill {
        return;
    }

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
        let empty_text = l10n
            .as_deref()
            .map(|l| l10n_or(l, "intel-quest-log-empty", "No active quests."))
            .unwrap_or_else(|| "No active quests.".to_string());
        let empty = commands
            .spawn((
                Text::new(empty_text),
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

/// Rebuild the faction standings column whenever the [`Player`]
/// resource changes, or on the first frame the panel exists
/// (so the initial fill happens even if `Player` was inserted
/// before the laptop UI was spawned — which is the normal
/// case, since the player resource comes from
/// `init_world_resources` on `OnEnter(AppState::Playing)` and
/// the laptop UI is built later on `OnEnter(PlayingState::Laptop)`).
fn refresh_faction_standings(
    mut commands: Commands,
    player: Res<Player>,
    l10n: Option<Res<Localization>>,
    font: Res<LaptopFont>,
    panel_q: Query<(Entity, Option<&Children>), With<FactionStandingsPanel>>,
    heading_q: Query<(), With<FactionStandingsHeading>>,
) {
    let Ok((panel_entity, panel_children)) = panel_q.single() else {
        return;
    };
    // "Needs first fill" == the panel exists but has only the
    // heading (or nothing). Any data rows mean we've populated
    // at least once and only need to rebuild on real changes.
    let non_heading_count = panel_children
        .map(|children| {
            children
                .iter()
                .filter(|c| heading_q.get(*c).is_err())
                .count()
        })
        .unwrap_or(0);
    let needs_first_fill = non_heading_count == 0;
    if !player.is_changed() && !needs_first_fill {
        return;
    }

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

    // Sort by faction ID so row order is stable across frames;
    // `PlayerState::new` seeds the list in `faction_ids()`
    // iteration order, which is `HashMap` order — visually
    // unstable. Clone + sort on display rather than on mutation.
    let mut rows: Vec<(Id<Faction>, Relation)> = player.0.standings.clone();
    rows.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

    for (faction, standing) in rows {
        let id_str = faction.as_str();
        let name_key = format!("faction-{id_str}");
        let display_name = l10n
            .as_deref()
            .map(|l| l10n_or(l, &name_key, id_str))
            .unwrap_or_else(|| id_str.to_string());

        let label_key = standing_label_key(standing);
        let label_fallback = standing_label_fallback(standing);
        let label = l10n
            .as_deref()
            .map(|l| l10n_or(l, label_key, label_fallback))
            .unwrap_or_else(|| label_fallback.to_string());
        let value = standing.value();
        let color = standing_color(standing);

        let row = commands
            .spawn((
                Text::new(format!("{display_name}: {label} ({value:+})")),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(color),
            ))
            .id();
        commands.entity(panel_entity).add_child(row);
    }
}

/// FTL key for a faction standing bucket. Mirrors the buckets
/// on [`Relation`] so the UI and the sim agree on what
/// "friendly" means.
fn standing_label_key(standing: Relation) -> &'static str {
    if standing.is_hostile() {
        "standing-hostile"
    } else if standing.is_unfriendly() {
        "standing-unfriendly"
    } else if standing.is_allied() {
        "standing-allied"
    } else if standing.is_friendly() {
        "standing-friendly"
    } else {
        "standing-neutral"
    }
}

/// English fallback used when [`Localization`] has not been
/// built yet (loading-state frames before the bundle is live).
fn standing_label_fallback(standing: Relation) -> &'static str {
    if standing.is_hostile() {
        "Hostile"
    } else if standing.is_unfriendly() {
        "Unfriendly"
    } else if standing.is_allied() {
        "Allied"
    } else if standing.is_friendly() {
        "Friendly"
    } else {
        "Neutral"
    }
}

/// Row text colour matching the standing bucket. Mild shifts
/// only — the intel panel is quiet by design, standings are
/// the most dynamic piece and don't need to scream.
fn standing_color(standing: Relation) -> Color {
    if standing.is_hostile() {
        Color::srgba(0.85, 0.35, 0.30, 0.95)
    } else if standing.is_unfriendly() {
        Color::srgba(0.80, 0.55, 0.35, 0.90)
    } else if standing.is_allied() {
        Color::srgba(0.45, 0.85, 0.55, 0.95)
    } else if standing.is_friendly() {
        Color::srgba(0.55, 0.80, 0.60, 0.90)
    } else {
        Color::srgba(0.70, 0.70, 0.70, 0.85)
    }
}
