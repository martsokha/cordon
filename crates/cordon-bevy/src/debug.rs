//! Debug overlay: FPS counter, entity count, diagnostics, world
//! inspector, and dev-only cheats.
//!
//! Cheat keys:
//! - **F1** — toggle the world inspector
//! - **F2** — push a hardcoded test visitor onto the queue
//! - **F3** — toggle map fog of war
//! - **F4** — cycle time-scale (1× → 4× → 16× → 64× → 1×)
//! - **F5** — toggle map edge-scroll panning
//!
//! The entire module is gated behind `cfg(debug_assertions)` at
//! the `mod debug;` declaration in `main.rs`, so nothing in here
//! (egui, the world inspector, cheat keys, dev shortcuts) exists
//! in release builds at all. That means we can use deps like
//! `bevy_inspector_egui` freely without paying for them in
//! shipping builds.

use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;

use crate::bunker::{Visitor, VisitorQueue};
use crate::laptop::fog::FogEnabled;
use crate::laptop::input::EdgeScrollEnabled;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
        // World inspector — toggle with F1. Egui plugin is required
        // for the inspector window; both are wired here so the rest of
        // the app stays unaware of the dev overlay.
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(
            WorldInspectorPlugin::new().run_if(resource_equals(InspectorVisible(true))),
        );
        app.insert_resource(InspectorVisible(false));
        app.insert_resource(TimeAcceleration::default());
        app.add_systems(Startup, spawn_fps_counter);
        app.add_systems(
            Update,
            (
                update_fps_counter,
                toggle_inspector,
                debug_push_visitor,
                cheat_toggle_fog,
                cheat_toggle_edge_scroll,
                cheat_cycle_time_scale,
                apply_time_scale,
            ),
        );
    }
}

/// Player-selected time scale. Applied to `Time<Virtual>` so every
/// sim system that reads `delta_secs()` accelerates in lockstep
/// (combat, movement, goals, throttles, fire cooldowns). Real-time
/// systems that should not accelerate (UI smoothing, camera lerp)
/// must read `Res<Time<Real>>` explicitly.
#[derive(Resource, Debug, Clone, Copy)]
struct TimeAcceleration {
    multiplier: f32,
}

impl Default for TimeAcceleration {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

/// Time-scale presets cycled by F4.
const TIME_SCALE_PRESETS: &[f32] = &[1.0, 4.0, 16.0, 64.0];

/// Push [`TimeAcceleration.multiplier`] into Bevy's virtual time.
/// Scales every sim system that reads `Res<Time>.delta_secs()`.
fn apply_time_scale(accel: Res<TimeAcceleration>, mut virt: ResMut<Time<Virtual>>) {
    if !accel.is_changed() {
        return;
    }
    virt.set_relative_speed(accel.multiplier.max(0.0));
}

/// F4 → cycle through [`TIME_SCALE_PRESETS`].
fn cheat_cycle_time_scale(keys: Res<ButtonInput<KeyCode>>, mut accel: ResMut<TimeAcceleration>) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }
    let current = accel.multiplier;
    // Next preset strictly greater than current; wrap to smallest.
    let next = TIME_SCALE_PRESETS
        .iter()
        .copied()
        .find(|&s| s > current + 0.01)
        .unwrap_or(TIME_SCALE_PRESETS[0]);
    accel.multiplier = next;
    info!("cheat: time scale → {next}×");
}

/// F3 → toggle map fog of war. Reveals every area and shows every
/// NPC/relic regardless of player line of sight.
fn cheat_toggle_fog(keys: Res<ButtonInput<KeyCode>>, mut fog: ResMut<FogEnabled>) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    fog.enabled = !fog.enabled;
    info!("cheat: fog {}", if fog.enabled { "on" } else { "off" });
}

/// F5 → toggle map edge-scroll panning. Off by default. The
/// real toggle will live in a settings menu later; this cheat
/// lets us exercise the feature during development without
/// touching code.
fn cheat_toggle_edge_scroll(keys: Res<ButtonInput<KeyCode>>, mut edge: ResMut<EdgeScrollEnabled>) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    edge.0 = !edge.0;
    info!("cheat: edge-scroll {}", if edge.0 { "on" } else { "off" });
}

/// F2 → push a hardcoded test visitor onto the queue. Stand-in for
/// the real day-cycle scheduler.
fn debug_push_visitor(keys: Res<ButtonInput<KeyCode>>, mut queue: ResMut<VisitorQueue>) {
    if !keys.just_pressed(KeyCode::F2) {
        return;
    }
    queue.0.push_back(Visitor {
        display_name: "Garrison Soldier".to_string(),
        faction: Id::<Faction>::new("faction_garrison"),
        yarn_node: "Visitor_Garrison_Greeting".to_string(),
    });
    info!("debug: queued test visitor");
}

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
struct InspectorVisible(bool);

fn toggle_inspector(keys: Res<ButtonInput<KeyCode>>, mut visible: ResMut<InspectorVisible>) {
    if keys.just_pressed(KeyCode::F1) {
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
