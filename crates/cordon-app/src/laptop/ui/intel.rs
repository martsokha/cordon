//! Intel tab: quest log + intel feed.
//!
//! The static panel layout is built once on [`spawn`]. Two
//! containers are tagged with marker components so per-frame
//! refresh systems can rebuild their children from live world
//! state:
//!
//! - [`QuestListPanel`] — rebuilt by [`refresh_quest_list`]
//!   whenever [`QuestLog`] changes.
//! - [`IntelFeedPanel`] — rebuilt by [`refresh_intel_feed`]
//!   whenever [`PlayerIntel`] changes.

use bevy::prelude::*;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::QuestLog;
use cordon_sim::resources::PlayerIntel;

use super::{LaptopTab, TabContent};
use crate::locale::L10n;
use crate::ui::UiFont;

/// Marker on the flex column that holds the quest-list entries.
/// Refreshed in-place by [`refresh_quest_list`].
#[derive(Component)]
pub struct QuestListPanel;

/// Marker on the heading row of the quest list so
/// [`refresh_quest_list`] knows which child to preserve when
/// rebuilding the list.
#[derive(Component)]
pub struct QuestListHeading;

/// Marker on the flex column that holds the intel-feed entries.
/// Refreshed in-place by [`refresh_intel_feed`].
#[derive(Component)]
pub struct IntelFeedPanel;

/// Marker on the heading row of the intel feed.
#[derive(Component)]
pub struct IntelFeedHeading;

/// Shared first-fill + rebuild helper used by the refresh systems.
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
        // log and player intel (inserted by cordon-sim's plugins)
        // and the game data catalog (inserted once `AppState`
        // leaves `Loading`). Without these guards the systems
        // run during the loading state and panic on missing
        // resources.
        app.add_systems(
            Update,
            (refresh_quest_list, refresh_intel_feed)
                .run_if(resource_exists::<QuestLog>)
                .run_if(resource_exists::<PlayerIntel>)
                .run_if(resource_exists::<GameDataResource>),
        );
    }
}

pub fn spawn(commands: &mut Commands, font: &Handle<Font>, l10n: &L10n, laptop_cam: Entity) {
    let quest_log_heading = l10n.get("intel-quest-log");
    let intel_feed_heading = l10n.get("intel-feed");

    commands
        .spawn((
            TabContent(LaptopTab::Intel),
            bevy::ui::UiTargetCamera(laptop_cam),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                top: Val::Px(40.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::new(Val::Px(16.0), Val::Px(16.0), Val::Px(16.0), Val::Px(16.0)),
                column_gap: Val::Px(8.0),
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

            // Right: intel feed.
            parent
                .spawn((
                    IntelFeedPanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(50.0),
                        row_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(8.0)),
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
}

/// Rebuild the quest-list column whenever [`QuestLog`] changes,
/// or on the first frame the panel exists (same lifecycle
/// caveat as [`refresh_intel_feed`]: `QuestLog` is inserted by
/// cordon-sim's `QuestPlugin` before the laptop UI is spawned,
/// so the `is_changed()` window has already closed by the time
/// this system first gets a panel to populate).
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

    // Show every completed entry under a subheading, newest-first.
    // No cap: the player should always be able to scroll back and
    // see what they've done across a run.
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

        for done in log.completed.iter().rev() {
            let def_id = done.def_id.as_str();
            let title = l10n.get(def_id);
            let marker = if done.success { "✓" } else { "✗" };
            let (title_color, hint_color) = if done.success {
                (
                    Color::srgba(0.55, 0.80, 0.60, 0.9),
                    Color::srgba(0.55, 0.80, 0.60, 0.7),
                )
            } else {
                (
                    Color::srgba(0.80, 0.55, 0.45, 0.9),
                    Color::srgba(0.80, 0.55, 0.45, 0.7),
                )
            };

            // Mirror the active-quest entry shape: one container
            // per completed quest, title on top, outcome flavour
            // below. Outcome hint key matches the active stage
            // hint scheme (`{quest_id}_stage_{stage_id}`) so
            // authoring uses one pattern for both.
            let entry = commands
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                })
                .id();
            commands.entity(panel_entity).add_child(entry);

            let title_node = commands
                .spawn((
                    Text::new(format!("[{marker}] {title}")),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(title_color),
                ))
                .id();
            commands.entity(entry).add_child(title_node);

            let outcome_key = format!("{def_id}_stage_{}", done.outcome_stage.as_str());
            let outcome_text = l10n.get(&outcome_key);
            let hint_node = commands
                .spawn((
                    Text::new(outcome_text),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(hint_color),
                ))
                .id();
            commands.entity(entry).add_child(hint_node);
        }
    }
}

/// Rebuild the intel feed whenever [`PlayerIntel`] changes.
/// Shows known intel entries sorted newest-first, each entry
/// a title line followed by a dimmer description line.
fn refresh_intel_feed(
    mut commands: Commands,
    intel: Res<PlayerIntel>,
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

    const TITLE_COLOR: Color = Color::srgba(0.85, 0.85, 0.85, 0.95);
    const DESC_COLOR: Color = Color::srgba(0.65, 0.65, 0.65, 0.8);

    // Show newest entries first, capped at 10.
    const MAX_ENTRIES: usize = 10;
    for entry in intel.entries.iter().rev().take(MAX_ENTRIES) {
        let id_str = entry.id.as_str();
        let title = l10n.get(&format!("{id_str}_title"));
        let description = l10n.get(&format!("{id_str}_description"));

        let entry_node = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(4.0), Val::Px(4.0)),
                ..default()
            })
            .id();
        commands.entity(panel_entity).add_child(entry_node);

        let title_node = commands
            .spawn((
                Text::new(title),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(TITLE_COLOR),
            ))
            .id();
        commands.entity(entry_node).add_child(title_node);

        let desc_node = commands
            .spawn((
                Text::new(description),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(DESC_COLOR),
            ))
            .id();
        commands.entity(entry_node).add_child(desc_node);
    }
}
