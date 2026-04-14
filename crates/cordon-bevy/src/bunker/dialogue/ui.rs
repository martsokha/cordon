//! Visual-novel-style dialogue overlay rendered while a visitor
//! conversation is active.
//!
//! Reads [`CurrentDialogue`] and shows a bottom-center text box with:
//! - speaker name (if any)
//! - the line text
//! - either a "Continue" button (plain line) or one button per option
//!
//! Clicking a button writes a [`DialogueChoice`] message that the
//! dialogue runtime forwards to the underlying `DialogueRunner`.

use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use crate::PlayingState;
use crate::bunker::FpsCamera;
use crate::bunker::resources::{CurrentDialogue, DialogueChoice};

pub(super) struct DialogueUiPlugin;

impl Plugin for DialogueUiPlugin {
    fn build(&self, app: &mut App) {
        // All three systems live in Update so spawn_dialogue_ui can
        // wait until the bunker camera exists before spawning the
        // panel — `OnEnter` system order is non-deterministic.
        // `DialogueUiSpawned` resource gates the spawn to a single
        // run.
        app.add_systems(
            Update,
            (
                spawn_dialogue_ui,
                sync_dialogue_ui,
                handle_choice_click,
                handle_choice_keys,
            )
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}

#[derive(Component)]
struct DialoguePanel;

#[derive(Component)]
struct DialogueSpeaker;

#[derive(Component)]
struct DialogueText;

#[derive(Component)]
struct DialogueChoicesRow;

/// One choice button. `Continue` is encoded as `index = None`.
#[derive(Component, Clone)]
struct DialogueChoiceButton {
    /// `None` → Continue, `Some(i)` → option index in the current
    /// `CurrentDialogue::Options` list. We store the index rather
    /// than `OptionId` so the UI doesn't have to import yarnspinner.
    index: Option<usize>,
    /// Whether the player can actually pick this option. Yarn marks
    /// options ineligible when their `<<if>>` condition is false;
    /// we render them dim and refuse clicks.
    available: bool,
}

#[derive(Resource, Default)]
struct DialogueUiSpawned;

const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.06, 0.92);
const TEXT_COLOR: Color = Color::srgba(0.92, 0.92, 0.92, 1.0);
const TEXT_COLOR_DISABLED: Color = Color::srgba(0.45, 0.45, 0.45, 1.0);
const SPEAKER_COLOR: Color = Color::srgba(1.0, 0.85, 0.5, 1.0);
const BUTTON_BG: Color = Color::srgba(0.10, 0.10, 0.14, 0.95);
const BUTTON_BG_HOVER: Color = Color::srgba(0.18, 0.18, 0.24, 0.95);

fn spawn_dialogue_ui(
    mut commands: Commands,
    spawned: Option<Res<DialogueUiSpawned>>,
    asset_server: Res<AssetServer>,
    fps_camera_q: Query<Entity, With<FpsCamera>>,
) {
    if spawned.is_some() {
        return;
    }
    // Wait until the bunker has spawned its camera. Without this we
    // race the room `OnEnter` systems on the first frame and the
    // panel ends up untargeted (defaulting to the laptop camera by
    // virtue of its higher `order`).
    let Ok(fps_camera) = fps_camera_q.single() else {
        return;
    };
    commands.insert_resource(DialogueUiSpawned);

    let font: Handle<Font> = asset_server.load("fonts/PTMono-Regular.ttf");

    commands
        .spawn((
            DialoguePanel,
            // Route this UI tree explicitly to the bunker 3D camera.
            // The laptop 2D camera has a higher `order` so without
            // this Bevy makes it the default UI target and the
            // dialogue panel ends up rendered into the laptop view
            // instead of the bunker view.
            UiTargetCamera(fps_camera),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(15.0),
                right: Val::Percent(15.0),
                bottom: Val::Px(48.0),
                min_height: Val::Px(120.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            GlobalZIndex(120),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                DialogueSpeaker,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(SPEAKER_COLOR),
            ));
            panel.spawn((
                DialogueText,
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
            panel.spawn((
                DialogueChoicesRow,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));
        });
}

/// Mirror `CurrentDialogue` into the visible UI. Re-spawns choice
/// buttons whenever the option set changes.
fn sync_dialogue_ui(
    current: Res<CurrentDialogue>,
    mut commands: Commands,
    mut panel_q: Query<&mut Visibility, With<DialoguePanel>>,
    mut speaker_q: Query<&mut Text, (With<DialogueSpeaker>, Without<DialogueText>)>,
    mut text_q: Query<&mut Text, (With<DialogueText>, Without<DialogueSpeaker>)>,
    row_q: Query<(Entity, Option<&Children>), With<DialogueChoicesRow>>,
    asset_server: Res<AssetServer>,
) {
    if !current.is_changed() {
        return;
    }

    let Ok(mut panel_vis) = panel_q.single_mut() else {
        return;
    };
    let Ok(mut speaker) = speaker_q.single_mut() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };
    let Ok((row_entity, row_children)) = row_q.single() else {
        return;
    };

    // Despawn any existing choice buttons before rebuilding.
    if let Some(children) = row_children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let font: Handle<Font> = asset_server.load("fonts/PTMono-Regular.ttf");

    match &*current {
        CurrentDialogue::Idle => {
            *panel_vis = Visibility::Hidden;
        }
        CurrentDialogue::Line {
            speaker: spk,
            text: line,
        } => {
            *panel_vis = Visibility::Visible;
            speaker.0 = spk.clone().unwrap_or_default();
            text.0 = line.clone();
            // One "Continue" button. Slot "1" so the 1-key works.
            spawn_choice_button(
                &mut commands,
                row_entity,
                font.clone(),
                "[1] ▸ Continue",
                DialogueChoiceButton {
                    index: None,
                    available: true,
                },
            );
        }
        CurrentDialogue::Options { lines } => {
            *panel_vis = Visibility::Visible;
            // Clear the speaker + text rows: option mode means we're
            // waiting on the player. Yarn fires PresentLine for the
            // line *before* options, so the previous line text is
            // already on screen — leave it as-is.
            for (i, opt) in lines.iter().enumerate() {
                let label = format!("[{}] ▸ {}", i + 1, opt.text);
                spawn_choice_button(
                    &mut commands,
                    row_entity,
                    font.clone(),
                    &label,
                    DialogueChoiceButton {
                        index: Some(i),
                        available: opt.available,
                    },
                );
            }
        }
    }
}

fn spawn_choice_button(
    commands: &mut Commands,
    parent: Entity,
    font: Handle<Font>,
    label: &str,
    marker: DialogueChoiceButton,
) {
    let text_color = if marker.available {
        TEXT_COLOR
    } else {
        TEXT_COLOR_DISABLED
    };
    let button = commands
        .spawn((
            marker,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BUTTON_BG),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(text_color),
            ));
        })
        .id();
    commands.entity(parent).add_child(button);
}

/// Read button interactions and convert them to dialogue choices.
fn handle_choice_click(
    current: Res<CurrentDialogue>,
    interactions: Query<
        (&Interaction, &DialogueChoiceButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut writer: MessageWriter<DialogueChoice>,
) {
    for (interaction, button, mut bg) in interactions {
        // Yarn-marked unavailable options are non-interactive — they
        // still render (dimmed) but don't accept clicks or hover.
        if !button.available {
            continue;
        }
        match interaction {
            Interaction::Pressed => {
                let choice = match button.index {
                    None => DialogueChoice::Continue,
                    Some(i) => match &*current {
                        CurrentDialogue::Options { lines } => {
                            if let Some(opt) = lines.get(i) {
                                DialogueChoice::Option { id: opt.id }
                            } else {
                                continue;
                            }
                        }
                        _ => continue,
                    },
                };
                writer.write(choice);
            }
            Interaction::Hovered => bg.0 = BUTTON_BG_HOVER,
            Interaction::None => bg.0 = BUTTON_BG,
        }
    }
}

/// Number-key shortcuts for dialogue choices. `1` always advances
/// a `Line` (there's only ever one "Continue" button), and
/// `1`..`9` pick the matching option in `Options` mode. Ineligible
/// Yarn options are skipped — pressing their number is a no-op,
/// same as clicking the dimmed button.
fn handle_choice_keys(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<CurrentDialogue>,
    mut writer: MessageWriter<DialogueChoice>,
) {
    let digit_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    let Some(pressed) = digit_keys.iter().position(|k| keys.just_pressed(*k)) else {
        return;
    };

    match &*current {
        CurrentDialogue::Idle => {}
        CurrentDialogue::Line { .. } => {
            if pressed == 0 {
                writer.write(DialogueChoice::Continue);
            }
        }
        CurrentDialogue::Options { lines } => {
            let Some(opt) = lines.get(pressed) else {
                return;
            };
            if !opt.available {
                return;
            }
            writer.write(DialogueChoice::Option { id: opt.id });
        }
    }
}
