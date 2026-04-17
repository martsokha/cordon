//! Intel tab: quest log, intel feed, faction standings.
//!
//! The static panel layout is built once on [`spawn`]. Three
//! containers are tagged with marker components so per-frame
//! refresh systems can rebuild their children from live world
//! state:
//!
//! - [`QuestListPanel`] — rebuilt by [`refresh_quest_list`]
//!   whenever [`QuestLog`] changes.
//! - [`FactionStandingsPanel`] — rebuilt by
//!   [`refresh_faction_standings`] whenever standings change.
//! - [`IntelFeedPanel`] — rebuilt by [`refresh_intel_feed`]
//!   whenever [`PlayerIntel`] changes.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::{Id, Relation};
use cordon_core::world::narrative::IntelCategory;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::QuestLog;
use cordon_sim::resources::{PlayerIntel, PlayerStandings};

use super::{LaptopTab, TabContent};
use crate::fonts::UiFont;
use crate::locale::L10n;

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

/// Marker on the flex column that holds the intel-feed entries.
/// Refreshed in-place by [`refresh_intel_feed`].
#[derive(Component)]
pub struct IntelFeedPanel;

/// Marker on the heading row of the intel feed.
#[derive(Component)]
pub struct IntelFeedHeading;

/// Shared first-fill + rebuild helper used by
/// [`refresh_quest_list`] and [`refresh_faction_standings`].
///
/// Returns the panel entity when the caller should rebuild —
/// either because `dirty == true` (a watched resource changed)
/// or because the panel currently has no data children beyond
/// its heading row (first-fill). In the rebuild case the
/// helper has already despawned the data children, so the
/// caller can start from a clean slate.
///
/// Returns `None` when the panel hasn't been spawned yet or
/// neither trigger fired — in which case the caller does
/// nothing.
fn prepare_panel_rebuild<P: Component, H: Component>(
    commands: &mut Commands,
    panel_q: &Query<(Entity, Option<&Children>), With<P>>,
    heading_q: &Query<(), With<H>>,
    dirty: bool,
) -> Option<Entity> {
    let (panel_entity, panel_children) = panel_q.single().ok()?;
    let non_heading_count = panel_children
        .map(|children| {
            children
                .iter()
                .filter(|c| heading_q.get(*c).is_err())
                .count()
        })
        .unwrap_or(0);
    let needs_first_fill = non_heading_count == 0;
    if !dirty && !needs_first_fill {
        return None;
    }
    if let Some(children) = panel_children {
        for child in children.iter() {
            if heading_q.get(child).is_ok() {
                continue;
            }
            commands.entity(child).despawn();
        }
    }
    Some(panel_entity)
}

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
            (
                refresh_quest_list,
                refresh_faction_standings,
                refresh_intel_feed,
            )
                .run_if(resource_exists::<QuestLog>)
                .run_if(resource_exists::<PlayerStandings>)
                .run_if(resource_exists::<PlayerIntel>)
                .run_if(resource_exists::<GameDataResource>),
        );
    }
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>, l10n: &L10n) {
    let quest_log_heading = l10n.get("intel-quest-log");
    let faction_standings_heading = l10n.get("intel-faction-standings");
    let intel_feed_heading = l10n.get("intel-feed");

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
                        IntelFeedPanel,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            IntelFeedHeading,
                            Text::new(intel_feed_heading),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.6)),
                        ));
                    });
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
    l10n: L10n,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<QuestListPanel>>,
    heading_q: Query<(), With<QuestListHeading>>,
) {
    let Some(panel_entity) =
        prepare_panel_rebuild(&mut commands, &panel_q, &heading_q, log.is_changed())
    else {
        return;
    };

    let font = font.0.clone();
    let catalog = &data.0;

    if log.active.is_empty() {
        let empty_text = l10n.get("intel-quest-log-empty");
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
    }

    for active in &log.active {
        let def_id = active.def_id.as_str();

        // FTL keys mirror the raw quest ID after the category-
        // prefix rename; stage hints use `{quest_id}_stage_{stage_id}`.
        let title = l10n.get(def_id);

        let stage_id = active.current_stage.as_str();
        let hint_key = format!("{def_id}_stage_{stage_id}");
        let hint_text = l10n.get(&hint_key);

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

    // Show the last MAX_COMPLETED completed entries under a
    // subheading so the player can review what's happened
    // recently. Tail-biased (most-recent first) because an
    // emerging history reads better that way than
    // chronologically forward.
    const MAX_COMPLETED: usize = 5;
    if !log.completed.is_empty() {
        let heading_text = l10n.get("intel-quest-log-completed");
        let heading = commands
            .spawn((
                Text::new(heading_text),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.55, 0.55, 0.45, 0.9)),
                Node {
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel_entity).add_child(heading);

        for done in log.completed.iter().rev().take(MAX_COMPLETED) {
            let def_id = done.def_id.as_str();
            let title = l10n.get(def_id);
            let marker = if done.success { "✓" } else { "✗" };
            let color = if done.success {
                Color::srgba(0.55, 0.80, 0.60, 0.85)
            } else {
                Color::srgba(0.80, 0.55, 0.45, 0.85)
            };
            let row = commands
                .spawn((
                    Text::new(format!("[{marker}] {title}")),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(color),
                ))
                .id();
            commands.entity(panel_entity).add_child(row);
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
    standings: Res<PlayerStandings>,
    l10n: L10n,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<FactionStandingsPanel>>,
    heading_q: Query<(), With<FactionStandingsHeading>>,
) {
    let Some(panel_entity) =
        prepare_panel_rebuild(&mut commands, &panel_q, &heading_q, standings.is_changed())
    else {
        return;
    };

    let font = font.0.clone();

    // Sort by faction ID so row order is stable across frames;
    // `PlayerState::new` seeds the list in `faction_ids()`
    // iteration order, which is `HashMap` order — visually
    // unstable. Clone + sort on display rather than on mutation.
    let mut rows: Vec<(Id<Faction>, Relation)> = standings.standings.clone();
    rows.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

    for (faction, standing) in rows {
        let id_str = faction.as_str();
        let display_name = l10n.get(id_str);

        let label_key = standing_label_key(standing);
        let label = l10n.get(label_key);
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

/// Rebuild the intel feed whenever [`PlayerIntel`] changes.
/// Shows known intel entries sorted newest-first, with a
/// category tag and colour per entry.
fn refresh_intel_feed(
    mut commands: Commands,
    intel: Res<PlayerIntel>,
    data: Res<GameDataResource>,
    l10n: L10n,
    font: Res<UiFont>,
    panel_q: Query<(Entity, Option<&Children>), With<IntelFeedPanel>>,
    heading_q: Query<(), With<IntelFeedHeading>>,
) {
    let Some(panel_entity) =
        prepare_panel_rebuild(&mut commands, &panel_q, &heading_q, intel.is_changed())
    else {
        return;
    };

    let font = font.0.clone();

    if intel.entries.is_empty() {
        let empty_text = l10n.get("intel-feed-empty");
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

    // Show newest entries first, capped at 10.
    const MAX_ENTRIES: usize = 10;
    for entry in intel.entries.iter().rev().take(MAX_ENTRIES) {
        let id_str = entry.id.as_str();
        let title_key = format!("intel.{id_str}.title");
        let title = l10n.get(&title_key);

        let (tag, color) = match data.0.intel.get(&entry.id) {
            Some(def) => (
                intel_category_tag(def.category),
                intel_category_color(def.category),
            ),
            None => ("???", Color::srgba(0.5, 0.5, 0.5, 0.7)),
        };

        let row = commands
            .spawn((
                Text::new(format!("[{tag}] {title}")),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(color),
            ))
            .id();
        commands.entity(panel_entity).add_child(row);
    }
}

/// Short uppercase tag for the intel category.
fn intel_category_tag(cat: IntelCategory) -> &'static str {
    match cat {
        IntelCategory::Faction => "FAC",
        IntelCategory::Environmental => "ENV",
        IntelCategory::Economic => "ECO",
        IntelCategory::Rumour => "RUM",
        IntelCategory::Mission => "MSN",
    }
}

/// Row colour per intel category.
fn intel_category_color(cat: IntelCategory) -> Color {
    match cat {
        IntelCategory::Faction => Color::srgba(0.65, 0.75, 0.90, 0.90),
        IntelCategory::Environmental => Color::srgba(0.70, 0.85, 0.55, 0.90),
        IntelCategory::Economic => Color::srgba(0.90, 0.80, 0.50, 0.90),
        IntelCategory::Rumour => Color::srgba(0.75, 0.65, 0.80, 0.85),
        IntelCategory::Mission => Color::srgba(0.85, 0.70, 0.50, 0.90),
    }
}
