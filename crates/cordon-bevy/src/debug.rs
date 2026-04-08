//! Debug overlay: FPS counter, entity count, diagnostics, world
//! inspector, and dev-only shortcuts (F2 → push a test visitor).

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;

use crate::bunker::{Visitor, VisitorQueue};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
        // World inspector — toggle with F12. Egui plugin is required
        // for the inspector window; both are wired here so the rest of
        // the app stays unaware of the dev overlay.
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(
            WorldInspectorPlugin::new().run_if(resource_equals(InspectorVisible(true))),
        );
        app.insert_resource(InspectorVisible(false));
        app.add_systems(Startup, spawn_fps_counter);
        app.add_systems(Update, (update_fps_counter, toggle_inspector));
        // Dev-only shortcuts that don't belong in shipping builds.
        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_push_visitor);
    }
}

/// F2 → push a hardcoded test visitor onto the queue. Stand-in for
/// the real day-cycle scheduler. Compiled out of release builds.
#[cfg(debug_assertions)]
fn debug_push_visitor(keys: Res<ButtonInput<KeyCode>>, mut queue: ResMut<VisitorQueue>) {
    if !keys.just_pressed(KeyCode::F2) {
        return;
    }
    queue.0.push_back(Visitor {
        display_name: "Garrison Soldier".to_string(),
        faction: Id::<Faction>::new("garrison"),
        yarn_node: "Visitor_Garrison_Greeting".to_string(),
    });
    info!("debug: queued test visitor");
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
struct InspectorVisible(bool);

fn toggle_inspector(keys: Res<ButtonInput<KeyCode>>, mut visible: ResMut<InspectorVisible>) {
    if keys.just_pressed(KeyCode::F12) {
        visible.0 = !visible.0;
    }
}

#[derive(Component)]
struct FpsText;

fn spawn_fps_counter(mut commands: Commands) {
    commands.spawn((
        FpsText,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Px(8.0),
            ..default()
        },
        Text::new("FPS: --"),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgba(0.0, 1.0, 0.0, 0.6)),
        GlobalZIndex(200),
    ));
}

fn update_fps_counter(
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut query: Query<&mut Text, With<FpsText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    for mut text in &mut query {
        text.0 = format!("FPS: {fps:.0}");
    }
}
