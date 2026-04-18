//! Shared visual constants and button builders for the three menu
//! overlays (main menu, pause, ending). Keeping these central so all
//! three screens look the same without each module reinventing the
//! same button styling.

use bevy::prelude::*;

pub const BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
pub const PANEL_BG: Color = Color::srgba(0.04, 0.04, 0.06, 0.95);
pub const BUTTON_BG: Color = Color::srgba(0.12, 0.12, 0.15, 0.9);
pub const BUTTON_BG_HOVER: Color = Color::srgba(0.20, 0.20, 0.25, 0.95);
pub const BUTTON_BG_DISABLED: Color = Color::srgba(0.08, 0.08, 0.10, 0.6);

pub const TEXT_PRIMARY: Color = Color::srgba(1.0, 1.0, 1.0, 0.95);
pub const TEXT_DIM: Color = Color::srgba(0.55, 0.55, 0.55, 1.0);

pub const TITLE_SIZE: f32 = 28.0;
pub const BUTTON_TEXT_SIZE: f32 = 14.0;
pub const BODY_SIZE: f32 = 13.0;

/// Marker so hover feedback knows a button is interactive.
#[derive(Component, Clone, Copy)]
pub struct MenuButton {
    pub enabled: bool,
}

/// Full-screen dim overlay — the root node for each menu screen.
pub fn overlay_root() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Centered vertical column holding title + buttons.
pub fn panel() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Px(48.0), Val::Px(32.0)),
        row_gap: Val::Px(12.0),
        min_width: Val::Px(320.0),
        ..default()
    }
}

/// Hover tint — runs on every frame for every menu button. Grey out
/// disabled buttons so the player can tell they can't click them.
pub fn update_button_hover(
    mut buttons: Query<(&Interaction, &MenuButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, btn, mut bg) in &mut buttons {
        if !btn.enabled {
            *bg = BackgroundColor(BUTTON_BG_DISABLED);
            continue;
        }
        *bg = BackgroundColor(match *interaction {
            Interaction::Hovered | Interaction::Pressed => BUTTON_BG_HOVER,
            Interaction::None => BUTTON_BG,
        });
    }
}

/// Spawn a button into `parent`. Returns the button entity so the
/// caller can tag it with a module-specific component for click
/// routing.
pub fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    label: &str,
    enabled: bool,
) -> Entity {
    let bg = if enabled {
        BUTTON_BG
    } else {
        BUTTON_BG_DISABLED
    };
    let text_color = if enabled { TEXT_PRIMARY } else { TEXT_DIM };
    parent
        .spawn((
            Button,
            MenuButton { enabled },
            Node {
                width: Val::Px(240.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font,
                    font_size: BUTTON_TEXT_SIZE,
                    ..default()
                },
                TextColor(text_color),
            ));
        })
        .id()
}

/// A single button in an overlay layout. `label` is the displayed
/// text; `enabled` toggles the greyed-out style and the Interaction
/// check in the caller's click handler; `attach` runs after spawn so
/// the caller can attach its own action marker component and any
/// disabled-button handler can skip the entity cleanly.
pub struct OverlayButton<'a> {
    pub label: &'a str,
    pub enabled: bool,
    /// Hook that fires after the button entity is spawned. Attach
    /// the caller's action-marker component here.
    pub attach: Box<dyn FnOnce(EntityCommands) + 'a>,
}

impl<'a> OverlayButton<'a> {
    pub fn new<F>(label: &'a str, attach: F) -> Self
    where
        F: FnOnce(EntityCommands) + 'a,
    {
        Self {
            label,
            enabled: true,
            attach: Box::new(attach),
        }
    }

    pub fn disabled(label: &'a str) -> Self {
        Self {
            label,
            enabled: false,
            attach: Box::new(|_| {}),
        }
    }
}

/// Shared overlay layout: full-screen dim background, centered
/// panel, title line, optional dim subtitle, optional dim body,
/// then a vertical column of buttons. Every caller passes its own
/// root-marker component so state transitions can despawn precisely
/// the overlay they own.
pub fn spawn_overlay(
    commands: &mut Commands,
    font: Handle<Font>,
    camera: Entity,
    root_marker: impl Bundle,
    title: &str,
    subtitle: Option<&str>,
    body: Option<&str>,
    buttons: Vec<OverlayButton<'_>>,
) {
    commands
        .spawn((
            root_marker,
            bevy::ui::UiTargetCamera(camera),
            overlay_root(),
            BackgroundColor(BG),
            GlobalZIndex(500),
        ))
        .with_children(|root| {
            root.spawn((panel(), BackgroundColor(PANEL_BG)))
                .with_children(|p| {
                    p.spawn((
                        Text::new(title.to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: TITLE_SIZE,
                            ..default()
                        },
                        TextColor(TEXT_PRIMARY),
                        Node {
                            margin: UiRect::bottom(Val::Px(
                                if subtitle.is_some() || body.is_some() {
                                    8.0
                                } else {
                                    24.0
                                },
                            )),
                            ..default()
                        },
                    ));
                    if let Some(sub) = subtitle {
                        p.spawn((
                            Text::new(sub.to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: BODY_SIZE,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                            Node {
                                margin: UiRect::bottom(Val::Px(24.0)),
                                ..default()
                            },
                        ));
                    }
                    if let Some(text) = body {
                        p.spawn((
                            Text::new(text.to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: BODY_SIZE,
                                ..default()
                            },
                            TextColor(TEXT_DIM),
                            Node {
                                max_width: Val::Px(320.0),
                                margin: UiRect::bottom(Val::Px(24.0)),
                                ..default()
                            },
                        ));
                    }
                    for button in buttons {
                        let entity = spawn_button(p, font.clone(), button.label, button.enabled);
                        let cmds = p.commands_mut().entity(entity);
                        (button.attach)(cmds);
                    }
                });
        });
}
