use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use cordon_sim::day::radio::RadioBroadcast;
use cordon_sim::plugin::prelude::LastDailyExpenses;

use crate::bunker::camera::FpsCamera;

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

#[derive(Resource)]
pub(super) struct IconAtlas {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

struct PendingToast {
    icon_index: usize,
    text: String,
    color: Color,
}

/// Internal queue filled by subscriber systems, drained by
/// [`spawn_toasts`] in the same frame.
#[derive(Resource, Default)]
pub(super) struct ToastQueue(Vec<PendingToast>);

impl ToastQueue {
    fn push(&mut self, icon_index: usize, text: impl Into<String>, color: Color) {
        self.0.push(PendingToast {
            icon_index,
            text: text.into(),
            color,
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
    queue.push(ICON_NEW_INTEL, label, Color::srgb(0.7, 0.85, 0.55));
}

pub(super) fn on_daily_expenses(expenses: Res<LastDailyExpenses>, mut queue: ResMut<ToastQueue>) {
    if !expenses.is_changed() {
        return;
    }
    let Some(report) = &expenses.0 else {
        return;
    };
    let total = report.total.value();
    if total == 0 {
        return;
    }
    queue.push(
        ICON_DAILY_SPENDING,
        format!("Daily expenses: {total}cr"),
        Color::srgb(0.9, 0.7, 0.4),
    );
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
                .with_color(toast.color),
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
                TextColor(toast.color),
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
                BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.75)),
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
                let mut c = tc.0.to_srgba();
                c.alpha = alpha;
                tc.0 = c.into();
            }
            if let Ok(mut img) = image_q.get_mut(child) {
                let mut c = img.color.to_srgba();
                c.alpha = alpha;
                img.color = c.into();
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
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.75 * alpha)),
        ));
    }
}
