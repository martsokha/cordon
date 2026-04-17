use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy_fluent::prelude::Localization;
use cordon_sim::day::payroll::DailyExpensesProcessed;
use cordon_sim::day::radio::RadioBroadcast;
use cordon_sim::quest::messages::{QuestFinished, QuestStarted, QuestUpdated, StandingChanged};

use crate::bunker::camera::FpsCamera;
use crate::locale::l10n_or;

const CELL_SIZE: u32 = 16;
const GRID_COLS: u32 = 8;
const GRID_ROWS: u32 = 8;
const ICON_DISPLAY_SIZE: f32 = 20.0;
const FONT_SIZE: f32 = 13.0;
const FADE_IN: f32 = 0.3;
const HOLD: f32 = 3.5;
const FADE_OUT: f32 = 0.8;
const TOAST_GAP: f32 = 4.0;
const MAX_VISIBLE: usize = 5;

const ICON_RELATION_UP: usize = 4;
const ICON_RELATION_DOWN: usize = 20;
const ICON_NEW_INTEL: usize = 34;
const ICON_DAILY_SPENDING: usize = 36;
const ICON_QUEST: usize = 34;

const TEXT_COLOR: Color = Color::srgba(0.85, 0.85, 0.85, 1.0);
const ICON_COLOR: Color = Color::WHITE;

#[derive(Resource)]
pub(super) struct IconAtlas {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

struct PendingToast {
    icon_index: usize,
    text: String,
}

#[derive(Resource, Default)]
pub(super) struct ToastQueue(Vec<PendingToast>);

impl ToastQueue {
    fn push(&mut self, icon_index: usize, text: impl Into<String>) {
        self.0.push(PendingToast {
            icon_index,
            text: text.into(),
        });
    }
}

#[derive(Component)]
pub(super) struct Toast {
    elapsed: f32,
    total: f32,
}

impl Toast {
    fn new() -> Self {
        Self {
            elapsed: 0.0,
            total: FADE_IN + HOLD + FADE_OUT,
        }
    }

    fn alpha(&self) -> f32 {
        if self.elapsed < FADE_IN {
            self.elapsed / FADE_IN
        } else if self.elapsed < FADE_IN + HOLD {
            1.0
        } else {
            let fade_progress = (self.elapsed - FADE_IN - HOLD) / FADE_OUT;
            (1.0 - fade_progress).max(0.0)
        }
    }
}

pub(super) fn load_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("icons/ui/arrows.png");
    let layout =
        TextureAtlasLayout::from_grid(UVec2::splat(CELL_SIZE), GRID_COLS, GRID_ROWS, None, None);
    let layout = layouts.add(layout);
    commands.insert_resource(IconAtlas { image, layout });
    commands.init_resource::<ToastQueue>();
}

pub(super) fn on_radio_broadcast(
    mut broadcasts: MessageReader<RadioBroadcast>,
    mut queue: ResMut<ToastQueue>,
) {
    let mut intel_count = 0usize;
    for msg in broadcasts.read() {
        intel_count += msg.intel.len();
    }
    if intel_count == 0 {
        return;
    }
    let label = if intel_count == 1 {
        "New intel received".to_string()
    } else {
        format!("{intel_count} new intel received")
    };
    queue.push(ICON_NEW_INTEL, label);
}

pub(super) fn on_daily_expenses(
    mut events: MessageReader<DailyExpensesProcessed>,
    mut queue: ResMut<ToastQueue>,
) {
    for msg in events.read() {
        let total = msg.total.value();
        queue.push(ICON_DAILY_SPENDING, format!("Daily expenses: {total}cr"));
    }
}

pub(super) fn on_standing_change(
    l10n: Option<Res<Localization>>,
    mut changes: MessageReader<StandingChanged>,
    mut queue: ResMut<ToastQueue>,
) {
    for msg in changes.read() {
        let name = l10n
            .as_deref()
            .map(|l| l10n_or(l, msg.faction.as_str(), msg.faction.as_str()))
            .unwrap_or_else(|| msg.faction.as_str().to_string());

        let delta = msg.delta.value();
        let (icon, sign) = if delta > 0 {
            (ICON_RELATION_UP, "+")
        } else {
            (ICON_RELATION_DOWN, "")
        };
        queue.push(icon, format!("{name}: {sign}{delta}"));
    }
}

pub(super) fn on_quest_started(
    l10n: Option<Res<Localization>>,
    mut started: MessageReader<QuestStarted>,
    mut queue: ResMut<ToastQueue>,
) {
    for msg in started.read() {
        let name = l10n
            .as_deref()
            .map(|l| l10n_or(l, msg.quest.as_str(), msg.quest.as_str()))
            .unwrap_or_else(|| msg.quest.as_str().to_string());
        queue.push(ICON_QUEST, format!("New quest: {name}"));
    }
}

pub(super) fn on_quest_updated(
    l10n: Option<Res<Localization>>,
    mut updated: MessageReader<QuestUpdated>,
    mut queue: ResMut<ToastQueue>,
) {
    for msg in updated.read() {
        let name = l10n
            .as_deref()
            .map(|l| l10n_or(l, msg.quest.as_str(), msg.quest.as_str()))
            .unwrap_or_else(|| msg.quest.as_str().to_string());
        queue.push(ICON_QUEST, format!("Quest updated: {name}"));
    }
}

pub(super) fn on_quest_finished(
    l10n: Option<Res<Localization>>,
    mut finished: MessageReader<QuestFinished>,
    mut queue: ResMut<ToastQueue>,
) {
    for msg in finished.read() {
        let name = l10n
            .as_deref()
            .map(|l| l10n_or(l, msg.quest.as_str(), msg.quest.as_str()))
            .unwrap_or_else(|| msg.quest.as_str().to_string());
        let status = if msg.success { "completed" } else { "failed" };
        queue.push(ICON_QUEST, format!("Quest {status}: {name}"));
    }
}

pub(super) fn spawn_toasts(
    mut commands: Commands,
    mut queue: ResMut<ToastQueue>,
    atlas: Res<IconAtlas>,
    camera_q: Query<Entity, With<FpsCamera>>,
    existing: Query<(), With<Toast>>,
) {
    if queue.0.is_empty() {
        return;
    }
    let Ok(camera) = camera_q.single() else {
        return;
    };

    let visible_count = existing.iter().count();
    let slots_left = MAX_VISIBLE.saturating_sub(visible_count);
    let take = slots_left.min(queue.0.len());

    for toast in queue.0.drain(..take) {
        let icon_node = commands
            .spawn((
                ImageNode::from_atlas_image(
                    atlas.image.clone(),
                    TextureAtlas {
                        layout: atlas.layout.clone(),
                        index: toast.icon_index,
                    },
                )
                .with_color(ICON_COLOR),
                Node {
                    width: Val::Px(ICON_DISPLAY_SIZE),
                    height: Val::Px(ICON_DISPLAY_SIZE),
                    ..default()
                },
            ))
            .id();

        let text_node = commands
            .spawn((
                Text::new(toast.text),
                TextFont {
                    font_size: FONT_SIZE,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ))
            .id();

        commands
            .spawn((
                Toast::new(),
                UiTargetCamera(camera),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(12.0),
                    top: Val::Px(12.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                GlobalZIndex(110),
            ))
            .add_children(&[icon_node, text_node]);
    }
}

pub(super) fn animate_toasts(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut toasts: Query<(Entity, &mut Toast, &Children)>,
    mut text_q: Query<&mut TextColor>,
    mut image_q: Query<&mut ImageNode>,
) {
    let dt = time.delta_secs();

    let mut sorted: Vec<(Entity, f32)> = Vec::new();

    for (entity, mut toast, children) in &mut toasts {
        toast.elapsed += dt;

        if toast.elapsed >= toast.total {
            commands.entity(entity).despawn();
            continue;
        }

        let alpha = toast.alpha();

        for child in children.iter() {
            if let Ok(mut tc) = text_q.get_mut(child) {
                tc.0 = Color::srgba(0.85, 0.85, 0.85, alpha);
            }
            if let Ok(mut img) = image_q.get_mut(child) {
                img.color = Color::srgba(1.0, 1.0, 1.0, alpha);
            }
        }

        sorted.push((entity, alpha));
    }

    sorted.sort_by_key(|(e, _)| *e);
    for (i, (entity, alpha)) in sorted.iter().enumerate() {
        commands.entity(*entity).insert((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0 + i as f32 * (ICON_DISPLAY_SIZE + TOAST_GAP + 8.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6 * alpha)),
        ));
    }
}
